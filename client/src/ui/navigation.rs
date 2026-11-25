//! Moving around the menus without a mouse.
//!
//! - Arrow keys, the D-pad or the left stick move the focus to the nearest widget in that
//!   direction (Bevy's automatic directional navigation), all three alike: once when pressed, then
//!   again and again while held (since v1.0.0-rc13; only the stick repeated before). Tab and
//!   Shift+Tab go through every widget in reading order, and wrap around. With nothing focused, or
//!   the focus on something gone, any of them focuses the first widget.
//! - The mouse leads while it moves: the focus follows it onto the widget under it, so the
//!   keyboard and the gamepad carry on from there, and the hover is drawn only while the mouse
//!   leads ([`PointerLeads`]), so a menu never shows two widgets lit, one by the mouse and one by
//!   the focus (since v1.0.0-rc13, the user's word on 2026-11-25: "Verifie et fix si besoin la
//!   navigation dans l'UI, le passage du focus [...] ca doit etre vraiment cohérent").
//! - Left and right step a focused [`Stepper`] instead, and stay inside a focused text field.
//! - Enter or the gamepad's A button activates the focused widget. Buttons handle Enter
//!   themselves; everything else gets an [`Activate`] from here.
//! - Escape or the gamepad's B button is "back", which each screen interprets.

use bevy::ecs::system::SystemParam;
use bevy::input::InputSystems;
use bevy::input::gamepad::GamepadEvent;
use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::input_focus::directional_navigation::DirectionalNavigationPlugin;
use bevy::input_focus::{FocusCause, InputFocus};
use bevy::math::CompassOctant;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::text::EditableText;
use bevy::ui::auto_directional_navigation::{AutoDirectionalNavigation, AutoDirectionalNavigator};
use bevy::ui::{ComputedNode, InteractionDisabled, UiGlobalTransform};
use bevy::ui_widgets::{Activate, Button};

use super::settings::Listening;
use super::widgets::Stepper;
use crate::screen::Screen;

/// How far the stick must lean to move the focus, and how fast holding a direction repeats.
const STICK_THRESHOLD: f32 = 0.55;
const REPEAT_DELAY: f64 = 0.4;
const REPEAT_INTERVAL: f64 = 0.12;
/// How much more a widget's sideways gap counts than its distance ahead, picking where a move
/// goes: a widget straight below wins over a nearer one off to the side.
const SIDEWAYS_WEIGHT: f32 = 3.0;

/// A dialog over the screen: while one is shown, the focus moves only among its widgets (since
/// v1.0.0-rc17). The one drawn on top wins when dialogs are stacked.
#[derive(Component, Default)]
pub struct Modal;

pub struct NavigationPlugin;

impl Plugin for NavigationPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(DirectionalNavigationPlugin)
            .init_resource::<LastInput>()
            .init_resource::<PointerLeads>()
            .add_systems(
                PreUpdate,
                (track_last_input, navigate).chain().after(InputSystems),
            )
            .add_systems(Update, focus_follows_pointer)
            .add_systems(PostUpdate, scroll_focus_into_view);
    }
}

/// Whether the mouse moved or was pressed since a key or a gamepad last moved the focus: the
/// widgets draw their hover only then.
#[derive(Resource, Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct PointerLeads(pub bool);

/// The device the player used last, so hints show the right buttons.
#[derive(Resource, Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum LastInput {
    #[default]
    Keyboard,
    Gamepad,
}

/// Menu actions from the keyboard and every gamepad at once.
#[derive(SystemParam)]
pub struct MenuInput<'w, 's> {
    keyboard: Res<'w, ButtonInput<KeyCode>>,
    gamepads: Query<'w, 's, &'static Gamepad>,
}

impl MenuInput<'_, '_> {
    pub fn back(&self) -> bool {
        self.keyboard.just_pressed(KeyCode::Escape) || self.gamepad(GamepadButton::East)
    }

    pub fn key(&self, key: KeyCode) -> bool {
        self.keyboard.just_pressed(key)
    }

    pub fn gamepad(&self, button: GamepadButton) -> bool {
        self.gamepads
            .iter()
            .any(|gamepad| gamepad.just_pressed(button))
    }
}

fn track_last_input(
    mut last_input: ResMut<LastInput>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut gamepad_events: MessageReader<GamepadEvent>,
) {
    let gamepad_used = gamepad_events.read().fold(false, |used, event| {
        used || match event {
            GamepadEvent::Button(button) => button.state.is_pressed(),
            GamepadEvent::Axis(axis) => axis.value.abs() > STICK_THRESHOLD,
            GamepadEvent::Connection(_) => false,
        }
    });
    let used = if gamepad_used {
        LastInput::Gamepad
    } else if keyboard.get_just_pressed().next().is_some()
        || mouse.get_just_pressed().next().is_some()
    {
        LastInput::Keyboard
    } else {
        return;
    };
    last_input.set_if_neq(used);
}

/// The direction held, from the keys, the D-pad or the stick, and when it moves the focus again.
#[derive(Default)]
struct Repeat {
    held: Option<CompassOctant>,
    next_at: f64,
}

fn navigate(
    input: MenuInput,
    mut navigator: AutoDirectionalNavigator,
    mut steppers: Query<&mut Stepper>,
    text_fields: Query<(), With<EditableText>>,
    buttons: Query<(), With<Button>>,
    disabled: Query<(), With<InteractionDisabled>>,
    widgets: Query<NavigableWidget, With<AutoDirectionalNavigation>>,
    time: Res<Time<Real>>,
    listening: Res<Listening>,
    mut repeat: Local<Repeat>,
    mut pointer: ResMut<PointerLeads>,
    screen: Option<Res<State<Screen>>>,
    (modals, parents): (
        Query<(Entity, &GlobalZIndex, &ComputedNode, &InheritedVisibility), With<Modal>>,
        Query<&ChildOf>,
    ),
    mut commands: Commands,
) {
    // The settings are waiting for a key or a button to bind: it is theirs, not the menus'.
    if listening.active() {
        return;
    }
    let focused = navigator.input_focus();
    let in_text = focused.is_some_and(|entity| text_fields.contains(entity));
    // The editor's arrows pan its map; its panel is still reached with the D-pad, the stick and Tab.
    let arrows = screen.is_none_or(|screen| *screen.get() != Screen::Editor);
    let shift =
        input.keyboard.pressed(KeyCode::ShiftLeft) || input.keyboard.pressed(KeyCode::ShiftRight);

    let direction = held_direction(
        &input,
        in_text,
        arrows,
        &mut repeat,
        time.elapsed_secs_f64(),
    );
    let tab = input.key(KeyCode::Tab);
    if direction.is_some() || tab {
        pointer.set_if_neq(PointerLeads(false));
    }
    // An open dialog keeps the focus inside it: the widgets behind it are covered.
    let scope = modals
        .iter()
        .filter(|(_, _, node, visibility)| !node.is_empty() && visibility.get())
        .max_by_key(|(_, z, _, _)| z.0)
        .map(|(entity, ..)| entity);
    let inside = |entity: Entity| {
        scope.is_none_or(|scope| {
            std::iter::successors(Some(entity), |&entity| {
                parents.get(entity).ok().map(ChildOf::parent)
            })
            .any(|ancestor| ancestor == scope)
        })
    };
    // What a move may land on: shown, usable, and inside the open dialog if there is one.
    let candidates = || -> Vec<Entity> {
        reading_order(&widgets)
            .into_iter()
            .filter(|&entity| !disabled.contains(entity) && inside(entity))
            .collect()
    };
    // Nothing focused, or the focus on a widget gone, hidden or under a dialog: the first widget
    // takes it, whatever moved it, instead of the press doing nothing.
    let order = || -> Vec<Entity> {
        reading_order(&widgets)
            .into_iter()
            .filter(|&entity| inside(entity) && (!disabled.contains(entity) || Some(entity) == focused))
            .collect()
    };
    let lost = focused.is_none_or(|entity| {
        !inside(entity)
            || widgets
                .get(entity)
                .map_or(true, |(_, node, _, visibility)| {
                    node.is_empty() || !visibility.get()
                })
    });
    if lost && (direction.is_some() || tab) {
        if let Some(&first) = order().first() {
            navigator
                .manual_directional_navigation
                .focus
                .set(first, FocusCause::Navigated);
        }
        return;
    }
    if tab {
        // Disabled widgets draw no focus: Tab goes past them, as the arrows do.
        let order = order();
        let next = match focused.and_then(|entity| order.iter().position(|&e| e == entity)) {
            Some(index) if shift => order.get((index + order.len() - 1) % order.len()),
            Some(index) => order.get((index + 1) % order.len()),
            None => order.first(),
        };
        if let Some(&next) = next {
            navigator
                .manual_directional_navigation
                .focus
                .set(next, FocusCause::Navigated);
        }
    }
    if let Some(octant) = direction {
        let step = match octant {
            CompassOctant::West => Some(-1),
            CompassOctant::East => Some(1),
            _ => None,
        };
        match (
            focused.and_then(|entity| steppers.get_mut(entity).ok()),
            step,
        ) {
            (Some(mut stepper), Some(step)) => stepper.step(step),
            _ => {
                // The widget to go to is picked here rather than by Bevy's navigation (since
                // v1.0.0-rc17, the user's word: "Il y a encore des deplacements dans l'UI a la
                // croix qui sont bizarres, n'arrivent sur rien"). Bevy's looked at every widget on
                // the screen, those behind an open dialog included, so the focus went under the
                // dialog where nothing showed it; and it took disabled ones, which draw no focus
                // (the user's report of 2026-11-23). Now only what can be seen and used.
                if let Some(start) = focused
                    && let Some(next) = pick_toward(start, octant, &candidates(), &widgets)
                {
                    navigator
                        .manual_directional_navigation
                        .focus
                        .set(next, FocusCause::Navigated);
                }
            }
        }
    }

    let Some(focused) = navigator.input_focus() else {
        return;
    };
    let enter = input.key(KeyCode::Enter) || input.key(KeyCode::NumpadEnter);
    // Buttons already handle Enter.
    if input.gamepad(GamepadButton::South) {
        pointer.set_if_neq(PointerLeads(false));
    }
    if input.gamepad(GamepadButton::South) || (enter && !buttons.contains(focused)) {
        commands.trigger(Activate { entity: focused });
    }
}

/// Every visible widget that takes the focus, as a page reads: rows from top to bottom, each from
/// left to right. A widget belongs to the row of the first one above it whose height spans its
/// center, so a small button beside a tall one still shares its row.
fn reading_order(widgets: &Query<NavigableWidget, With<AutoDirectionalNavigation>>) -> Vec<Entity> {
    let mut areas: Vec<(Entity, Vec2, f32)> = widgets
        .iter()
        .filter(|(_, node, _, visibility)| !node.is_empty() && visibility.get())
        .map(|(entity, node, transform, _)| (entity, transform.translation, node.size().y))
        .collect();
    areas.sort_by(|a, b| a.1.y.total_cmp(&b.1.y));

    let mut order = Vec::with_capacity(areas.len());
    let mut rest = areas.as_slice();
    while let Some(&(_, first_center, first_height)) = rest.first() {
        let bottom = first_center.y + first_height / 2.0;
        let len = rest
            .iter()
            .position(|(_, center, _)| center.y > bottom)
            .unwrap_or(rest.len());
        let mut row = rest[..len].to_vec();
        row.sort_by(|a, b| a.1.x.total_cmp(&b.1.x));
        order.extend(row.into_iter().map(|(entity, _, _)| entity));
        rest = &rest[len..];
    }
    order
}

/// Where a move from `start` toward `octant` goes among `candidates`, by where they are drawn.
fn pick_toward(
    start: Entity,
    octant: CompassOctant,
    candidates: &[Entity],
    widgets: &Query<NavigableWidget, With<AutoDirectionalNavigation>>,
) -> Option<Entity> {
    let rect = |entity: Entity| {
        widgets
            .get(entity)
            .ok()
            .map(|(_, node, transform, _)| Rect::from_center_size(transform.translation, node.size()))
    };
    let from = rect(start)?;
    let others: Vec<(Entity, Rect)> = candidates
        .iter()
        .filter(|&&entity| entity != start)
        .filter_map(|&entity| Some((entity, rect(entity)?)))
        .collect();
    best_toward(from, octant, &others)
}

/// The nearest of `others` toward `octant` from `from`, in screen coordinates (y down): ahead of
/// it, and lined up with it rather than off to the side, a widget overlapping its column (or row)
/// counting as lined up.
fn best_toward(from: Rect, octant: CompassOctant, others: &[(Entity, Rect)]) -> Option<Entity> {
    let direction = match octant {
        CompassOctant::North => Vec2::NEG_Y,
        CompassOctant::South => Vec2::Y,
        CompassOctant::West => Vec2::NEG_X,
        CompassOctant::East => Vec2::X,
        _ => return None,
    };
    let across = direction.perp().abs();
    others
        .iter()
        .filter_map(|&(entity, rect)| {
            let offset = rect.center() - from.center();
            let ahead = offset.dot(direction);
            // Ahead of the start's own edge, not merely of its center.
            let reach = (from.half_size() + rect.half_size()).dot(direction.abs()) * 0.4;
            if ahead <= reach {
                return None;
            }
            let sideways = offset.dot(across).abs();
            let overlap = (from.half_size() + rect.half_size()).dot(across);
            let gap = (sideways - overlap).max(0.0);
            Some((entity, ahead + SIDEWAYS_WEIGHT * gap + 0.1 * sideways))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(entity, _)| entity)
}

/// The focus moved to a widget a list has scrolled out of sight: the list scrolls to show it.
fn scroll_focus_into_view(
    focus: Res<InputFocus>,
    mut last: Local<Option<Entity>>,
    nodes: Query<(&ComputedNode, &UiGlobalTransform)>,
    parents: Query<&ChildOf>,
    mut scrollers: Query<(&mut ScrollPosition, &Node)>,
) {
    let now = focus.get();
    if now == *last {
        return;
    }
    *last = now;
    let Some(focused) = now else {
        return;
    };
    let Ok((node, transform)) = nodes.get(focused) else {
        return;
    };
    let (top, bottom) = (
        transform.translation.y - node.size().y / 2.0,
        transform.translation.y + node.size().y / 2.0,
    );
    let ancestors = std::iter::successors(parents.get(focused).ok().map(ChildOf::parent), |&e| {
        parents.get(e).ok().map(ChildOf::parent)
    });
    for ancestor in ancestors {
        let Ok((mut scroll, style)) = scrollers.get_mut(ancestor) else {
            continue;
        };
        if style.overflow.y != OverflowAxis::Scroll {
            continue;
        }
        let Ok((list, list_transform)) = nodes.get(ancestor) else {
            return;
        };
        let (list_top, list_bottom) = (
            list_transform.translation.y - list.size().y / 2.0,
            list_transform.translation.y + list.size().y / 2.0,
        );
        // Physical pixels on screen, logical in the scroll position.
        let shift = if top < list_top {
            top - list_top
        } else if bottom > list_bottom {
            bottom - list_bottom
        } else {
            0.0
        };
        if shift != 0.0 {
            scroll.y = (scroll.y + shift * list.inverse_scale_factor()).max(0.0);
        }
        return;
    }
}

type NavigableWidget = (
    Entity,
    &'static ComputedNode,
    &'static UiGlobalTransform,
    &'static InheritedVisibility,
);

/// A direction when an arrow key, the D-pad or the stick is first pushed, then repeatedly while it
/// stays pushed: the three move the focus alike. Left and right arrows are the text's in a text
/// field.
fn held_direction(
    input: &MenuInput,
    in_text: bool,
    arrows: bool,
    repeat: &mut Repeat,
    now: f64,
) -> Option<CompassOctant> {
    const DIRECTIONS: [(KeyCode, GamepadButton, CompassOctant); 4] = [
        (
            KeyCode::ArrowUp,
            GamepadButton::DPadUp,
            CompassOctant::North,
        ),
        (
            KeyCode::ArrowDown,
            GamepadButton::DPadDown,
            CompassOctant::South,
        ),
        (
            KeyCode::ArrowLeft,
            GamepadButton::DPadLeft,
            CompassOctant::West,
        ),
        (
            KeyCode::ArrowRight,
            GamepadButton::DPadRight,
            CompassOctant::East,
        ),
    ];
    let of_key = |key: KeyCode, octant: CompassOctant| {
        arrows
            && !(in_text && matches!(octant, CompassOctant::West | CompassOctant::East))
            && input.keyboard.pressed(key)
    };
    let of_pad = |button: GamepadButton| input.gamepads.iter().any(|pad| pad.pressed(button));
    // A press this frame wins, even one let go within the frame.
    let pressed_now = DIRECTIONS.iter().find_map(|&(key, button, octant)| {
        let key_now = arrows
            && input.key(key)
            && !(in_text && matches!(octant, CompassOctant::West | CompassOctant::East));
        (key_now || input.gamepad(button)).then_some(octant)
    });
    let lean = input
        .gamepads
        .iter()
        .map(Gamepad::left_stick)
        .max_by(|a, b| a.length_squared().total_cmp(&b.length_squared()))
        .unwrap_or(Vec2::ZERO);
    let held = if let Some(&(_, _, octant)) = DIRECTIONS
        .iter()
        .find(|&&(key, button, octant)| of_key(key, octant) || of_pad(button))
    {
        Some(octant)
    } else if lean.length() < STICK_THRESHOLD {
        None
    } else if lean.x.abs() > lean.y.abs() {
        Some(if lean.x > 0.0 {
            CompassOctant::East
        } else {
            CompassOctant::West
        })
    } else {
        Some(if lean.y > 0.0 {
            CompassOctant::North
        } else {
            CompassOctant::South
        })
    };
    repeat.next(pressed_now, held, now)
}

impl Repeat {
    /// The direction to move the focus in at `now`: `pressed_now`, one just pressed, at once; else
    /// `held`, the one held, when it is new and then again every interval after a delay.
    fn next(
        &mut self,
        pressed_now: Option<CompassOctant>,
        held: Option<CompassOctant>,
        now: f64,
    ) -> Option<CompassOctant> {
        if let Some(octant) = pressed_now {
            self.held = Some(octant);
            self.next_at = now + REPEAT_DELAY;
            return Some(octant);
        }
        if held != self.held {
            self.held = held;
            self.next_at = now + REPEAT_DELAY;
            return held;
        }
        if held.is_some() && now >= self.next_at {
            self.next_at = now + REPEAT_INTERVAL;
            return held;
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A direction moves the focus once when pressed, then, held, again after a delay and every
    /// interval; a press let go within a frame still counts, and so does a fresh press of the same
    /// direction right after.
    #[test]
    fn a_move_goes_to_what_is_lined_up_ahead() {
        let at = |x: f32, y: f32, w: f32, h: f32| Rect::from_center_size(Vec2::new(x, y), Vec2::new(w, h));
        let mut world = World::new();
        let [below, below_wide, beside, far_side, above] =
            [(); 5].map(|_| world.spawn_empty().id());
        let from = at(100.0, 100.0, 80.0, 30.0);
        let others = [
            (below, at(100.0, 160.0, 80.0, 30.0)),
            (below_wide, at(300.0, 140.0, 80.0, 30.0)),
            (beside, at(200.0, 100.0, 80.0, 30.0)),
            (far_side, at(400.0, 105.0, 80.0, 30.0)),
            (above, at(110.0, 40.0, 200.0, 30.0)),
        ];
        // Down: the one in its column, though another is nearer but off to the side.
        assert_eq!(best_toward(from, CompassOctant::South, &others), Some(below));
        // Right: the nearer of the two in its row; nothing of a row below.
        assert_eq!(best_toward(from, CompassOctant::East, &others), Some(beside));
        // Up: the wide one overlapping its column.
        assert_eq!(best_toward(from, CompassOctant::North, &others), Some(above));
        // Left: nothing there, so the focus stays.
        assert_eq!(best_toward(from, CompassOctant::West, &others), None);
    }

    #[test]
    fn a_held_direction_repeats_as_the_stick_does() {
        let mut repeat = Repeat::default();
        let north = Some(CompassOctant::North);
        assert_eq!(repeat.next(north, north, 0.0), north);
        assert_eq!(repeat.next(None, north, 0.1), None);
        assert_eq!(repeat.next(None, north, REPEAT_DELAY), north);
        assert_eq!(repeat.next(None, north, REPEAT_DELAY + 0.01), None);
        assert_eq!(
            repeat.next(None, north, REPEAT_DELAY + REPEAT_INTERVAL),
            north
        );
        // Let go, then a tap pressed and released within one frame.
        assert_eq!(repeat.next(None, None, 1.0), None);
        assert_eq!(repeat.next(north, None, 1.02), north);
        // Pressed again at once.
        assert_eq!(repeat.next(north, north, 1.04), north);
        // Another direction held takes over at once.
        let east = Some(CompassOctant::East);
        assert_eq!(repeat.next(None, east, 1.06), east);
    }
}

/// While the mouse moves, the focus follows it onto the widget under it (not a text field, which
/// takes the focus when clicked, so typing is never taken away by a mouse passing over another).
fn focus_follows_pointer(
    motion: Res<AccumulatedMouseMotion>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut pointer: ResMut<PointerLeads>,
    mut focus: ResMut<InputFocus>,
    hovered: Query<(Entity, &Hovered), With<AutoDirectionalNavigation>>,
    disabled: Query<(), With<InteractionDisabled>>,
    text_fields: Query<(), With<EditableText>>,
    listening: Res<Listening>,
) {
    if motion.delta == Vec2::ZERO && mouse.get_just_pressed().next().is_none() {
        return;
    }
    pointer.set_if_neq(PointerLeads(true));
    if listening.active()
        || focus
            .get()
            .is_some_and(|entity| text_fields.contains(entity))
    {
        return;
    }
    if let Some((entity, _)) = hovered.iter().find(|(entity, hovered)| {
        hovered.get() && !disabled.contains(*entity) && !text_fields.contains(*entity)
    }) && focus.get() != Some(entity)
    {
        focus.set(entity, FocusCause::Navigated);
    }
}
