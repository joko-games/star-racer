//! The garage: where the player picks their car, paints it, and chooses its trail and its horn.
//!
//! Reached from the first screen, connected or not. The car stands over the menu's neon grid,
//! turning slowly with its trail swept out behind it, to the right of a card of six rows: the car,
//! its paint, its neon, its trail, the trail's color and the horn. The car, the trail and the horn
//! are stepped through with left and right, the arrows beside them, or a click, and what is not
//! won yet says at which level it will be (see `protocol::rewards`); the colors are palettes, every
//! color in a row at once, picked with a click or stepped along the same way. The player's level
//! stands under the title, small. What is chosen, of what is unlocked, is kept with the menu
//! choices and sent to the server, which shows it to everyone in the next lobby the player enters.
//! Every car drives the same: this is how a car looks and sounds, never how it handles.

use bevy::input_focus::{FocusCause, InputFocus};
use bevy::light::NotShadowCaster;
use bevy::prelude::*;
use bevy::ui_widgets::Activate;
use star_racer_protocol::rewards::{self, Reward};
use star_racer_protocol::{
    CAR_HORNS, CAR_MODELS, CAR_NEONS, CAR_PAINTS, CAR_TRAIL_COLORS, CAR_TRAILS, CAR_TURBOS, CarLook,
};

use super::navigation::MenuInput;
use super::safe_area::SafePadding;
use super::theme::{self, Fonts};
use super::touch::Touchscreen;
use super::widgets::{ButtonKind, Selectable, Stepper, button, hint, hint_row, panel, step_button};
use crate::car_model::{CarModel, NEONS, PAINTS};
use crate::lobby::PlayerProgress;
use crate::net::Network;
use crate::preferences::{LookPreference, StoredPreferences};
use crate::race::{CarAssets, spawn_car};
use crate::screen::Screen;
use crate::settings::Script;
use crate::sound::Sounds;
use crate::sound::horn::HORNS;
use crate::trail::{self, Effect, TRAIL_COLORS, TRAILS, TrailMaterial};

/// Where the car stands in front of the menu camera, which `backdrop.rs` places at 3 m over the
/// grid looking far ahead: a little to the right of the card, close enough to fill the screen.
const PREVIEW_AT: Vec3 = Vec3::new(2.6, 0.0, -8.5);
/// How fast it turns, in radians per second.
const TURN_RATE: f32 = 0.45;

pub struct GaragePlugin;

impl Plugin for GaragePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(Screen::Login), open_scripted)
            .add_systems(OnEnter(Screen::Garage), spawn_garage)
            .add_observer(pick_swatch)
            .add_observer(press_swatch)
            .add_systems(
                Update,
                (
                    show_choices,
                    show_swatches,
                    hover_swatches,
                    show_level,
                    show_preview,
                    play_horn,
                    preview_turbo,
                    turn_preview,
                    leave,
                )
                    .chain()
                    .run_if(in_state(Screen::Garage)),
            );
    }
}

/// Which part of the look a row steps through.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
enum Part {
    Model,
    Paint,
    Neon,
    Trail,
    TrailColor,
    Horn,
    /// The turbo skin (since v0.1.10).
    Turbo,
}

impl Part {
    /// Whether the row is a palette of colors rather than a list of names.
    fn is_palette(self) -> bool {
        matches!(self, Self::Paint | Self::Neon | Self::TrailColor)
    }

    /// The reward the row's `value` is, if the row holds things to win.
    fn reward(self, value: u8) -> Option<Reward> {
        match self {
            Self::Model => Some(Reward::Car(value)),
            Self::Trail => Some(Reward::Trail(value)),
            Self::Horn => Some(Reward::Horn(value)),
            Self::Turbo => Some(Reward::Turbo(value)),
            _ => None,
        }
    }

    /// The colors of a palette row, as they are seen: neon brought down to what a screen shows.
    fn colors(self) -> Vec<Color> {
        let seen = |neon: LinearRgba| {
            let max = neon.red.max(neon.green).max(neon.blue);
            Color::LinearRgba(neon * (1.0 / max)).with_alpha(1.0)
        };
        match self {
            Self::Paint => PAINTS.iter().map(|(_, color)| *color).collect(),
            Self::Neon => NEONS.iter().map(|(_, neon)| seen(*neon)).collect(),
            Self::TrailColor => TRAIL_COLORS.iter().map(|(_, neon)| seen(*neon)).collect(),
            _ => Vec::new(),
        }
    }
}

/// The name of what a row holds, and the line under it when it is locked.
#[derive(Component)]
struct ChoiceName(Part);

#[derive(Component)]
struct LockLine(Part);

/// A color of a palette row: a click on it picks it.
#[derive(Component)]
struct Swatch {
    row: Entity,
    value: u8,
    color: Color,
}

/// The line describing the car.
#[derive(Component)]
struct Blurb;

/// The player's level, under the title.
#[derive(Component)]
struct LevelLine;

/// The car turning beside the card, the look it is drawn in, and the mesh of its trail.
#[derive(Component)]
struct Preview(CarLook, Handle<Mesh>);

/// The look as the rows now say it, whether or not it is unlocked.
fn look_of(rows: &Query<(&Part, &Stepper)>) -> CarLook {
    let mut look = CarLook::default();
    for (part, stepper) in rows {
        match part {
            Part::Model => look.model = stepper.value,
            Part::Paint => look.paint = stepper.value,
            Part::Neon => look.neon = stepper.value,
            Part::Trail => look.trail = stepper.value,
            Part::TrailColor => look.trail_color = stepper.value,
            Part::Horn => look.horn = stepper.value,
            Part::Turbo => look.turbo = stepper.value,
        }
    }
    look.checked()
}

fn xp_of(progress: &PlayerProgress) -> u32 {
    progress.xp.unwrap_or(0)
}

fn spawn_garage(
    mut commands: Commands,
    fonts: Res<Fonts>,
    preferences: Res<StoredPreferences>,
    touchscreen: Res<Touchscreen>,
    mut focus: ResMut<InputFocus>,
) {
    let pick = |desk: f32, phone: f32| touchscreen.pick(desk, phone);
    let look = preferences.get().car_look();

    let card = commands
        .spawn((
            Node {
                width: px(pick(460.0, 400.0)),
                flex_direction: FlexDirection::Column,
                padding: px(pick(22.0, 10.0)).all(),
                row_gap: px(pick(5.0, 3.0)),
                border: px(1).all(),
                border_radius: BorderRadius::all(px(theme::RADIUS)),
                ..default()
            },
            panel(),
        ))
        .with_children(|card| {
            card.spawn(Node {
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Baseline,
                ..default()
            })
            .with_children(|title| {
                title.spawn((
                    Text::new("GARAGE"),
                    fonts.display(pick(40.0, 26.0)),
                    TextColor(theme::TEXT),
                    TextShadow {
                        offset: Vec2::new(3.0, 3.0),
                        color: theme::NEON.with_alpha(0.8),
                    },
                ));
                title.spawn((
                    LevelLine,
                    Text::default(),
                    fonts.caption(12.0),
                    theme::CAPTION_SPACING,
                    TextColor(theme::TEXT_FAINT),
                ));
            });
            card.spawn((
                Blurb,
                Text::default(),
                fonts.text(pick(14.0, 12.0), 400),
                TextColor(theme::TEXT_DIM),
                Node {
                    min_height: px(pick(20.0, 15.0)),
                    ..default()
                },
            ));
        })
        .id();

    let mut first_row = None;
    // In three groups under titles, the car, its trail, its effects (since v0.1.23, the user's word on
    // 2026-11-24: "Améliore le garage, ca manque un peu de clarté"): the rows' names were faint and
    // small, and COLOR did not say it was the trail's. The titles only on a desk, a phone's screen
    // having no height to spare.
    for (group, part, label, value, count) in [
        (Some("THE CAR"), Part::Model, "MODEL", look.model, CAR_MODELS),
        (None, Part::Paint, "PAINT", look.paint, CAR_PAINTS),
        (None, Part::Neon, "NEON", look.neon, CAR_NEONS),
        (Some("ITS TRAIL"), Part::Trail, "STYLE", look.trail, CAR_TRAILS),
        (None, Part::TrailColor, "COLOR", look.trail_color, CAR_TRAIL_COLORS),
        (Some("ITS EFFECTS"), Part::Horn, "HORN", look.horn, CAR_HORNS),
        (None, Part::Turbo, "TURBO", look.turbo, CAR_TURBOS),
    ] {
        if let Some(group) = group
            && !touchscreen.0
        {
            commands.entity(card).with_child((
                Text::new(group),
                fonts.caption(13.0),
                theme::CAPTION_SPACING,
                TextColor(theme::NEON),
                Node {
                    margin: UiRect::top(px(4)),
                    ..default()
                },
            ));
        }
        let row = commands
            .spawn((
                part,
                Stepper {
                    value,
                    min: 0,
                    max: count - 1,
                    wrap: true,
                },
                Selectable,
                Node {
                    padding: UiRect::axes(px(pick(12.0, 8.0)), px(pick(4.0, 2.0))),
                    align_items: AlignItems::Center,
                    column_gap: px(8),
                    border: px(1.5).all(),
                    border_radius: BorderRadius::all(px(theme::RADIUS_SMALL)),
                    ..default()
                },
            ))
            .id();
        first_row.get_or_insert(row);
        commands.entity(row).with_children(|content| {
            content.spawn((
                Text::new(label),
                fonts.caption(pick(13.0, 11.5)),
                theme::CAPTION_SPACING,
                TextColor(theme::TEXT_DIM),
                Node {
                    width: px(pick(60.0, 52.0)),
                    flex_shrink: 0.0,
                    ..default()
                },
            ));
            if part.is_palette() {
                // The chosen color's name, then every color at once as tiles, the chosen one
                // ringed in white and glowing in its own color (since v0.1.71; small dots before).
                content
                    .spawn(Node {
                        flex_grow: 1.0,
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        row_gap: px(pick(5.0, 3.0)),
                        padding: UiRect::vertical(px(pick(4.0, 2.0))),
                        ..default()
                    })
                    .with_children(|column| {
                        column.spawn((
                            ChoiceName(part),
                            Text::default(),
                            fonts.caption(pick(13.0, 11.0)),
                            theme::CAPTION_SPACING,
                            TextColor(theme::TEXT),
                        ));
                        column
                            .spawn(Node {
                                flex_wrap: FlexWrap::Wrap,
                                justify_content: JustifyContent::Center,
                                column_gap: px(pick(6.0, 4.0)),
                                row_gap: px(pick(6.0, 4.0)),
                                max_width: px(pick(252.0, 196.0)),
                                ..default()
                            })
                            .with_children(|palette| {
                                for (value, color) in part.colors().into_iter().enumerate() {
                                    palette.spawn((
                                        Swatch {
                                            row,
                                            value: value as u8,
                                            color,
                                        },
                                        Button,
                                        bevy::picking::hover::Hovered::default(),
                                        Node {
                                            width: px(pick(24.0, 20.0)),
                                            height: px(pick(24.0, 20.0)),
                                            border: px(2).all(),
                                            border_radius: BorderRadius::all(px(6)),
                                            ..default()
                                        },
                                        BackgroundColor(color),
                                        BorderColor::all(Color::BLACK.with_alpha(0.35)),
                                        BoxShadow(Vec::new()),
                                    ));
                                }
                            });
                    });
                return;
            }
            content.spawn(step_button(&fonts, row, -1));
            content
                .spawn(Node {
                    flex_grow: 1.0,
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                })
                .with_children(|middle| {
                    middle.spawn((
                        ChoiceName(part),
                        Text::default(),
                        fonts.text(pick(18.0, 15.0), 700),
                        TextColor(theme::TEXT),
                    ));
                    middle.spawn((
                        LockLine(part),
                        Text::default(),
                        fonts.caption(10.0),
                        theme::CAPTION_SPACING,
                        TextColor(theme::AMBER),
                        Node {
                            display: Display::None,
                            ..default()
                        },
                    ));
                });
            content.spawn(step_button(&fonts, row, 1));
        });
        commands.entity(card).add_child(row);
    }

    commands.entity(card).with_children(|card| {
        card.spawn(button(&fonts, ButtonKind::Primary, "DONE"))
            .insert(Node {
                height: px(pick(48.0, 38.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: px(1.5).all(),
                border_radius: BorderRadius::all(px(theme::RADIUS_SMALL)),
                margin: UiRect::top(px(pick(6.0, 2.0))),
                ..default()
            })
            .observe(done);
    });

    commands
        .spawn((
            DespawnOnExit(Screen::Garage),
            SafePadding::default(),
            Node {
                width: percent(100),
                height: percent(100),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                padding: UiRect::left(px(pick(60.0, 20.0))),
                ..default()
            },
        ))
        .add_child(card)
        .with_children(|root| {
            root.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    right: px(28),
                    bottom: px(22),
                    display: touchscreen.pick(Display::Flex, Display::None),
                    ..default()
                },
                children![(
                    hint_row(),
                    children![
                        hint(&fonts, "↑ ↓", "D-pad", "Choose"),
                        hint(&fonts, "← →", "D-pad", "Change"),
                        hint(&fonts, "Esc", "B", "Done"),
                    ],
                )],
            ));
        });

    commands.spawn((
        DespawnOnExit(Screen::Garage),
        PointLight {
            intensity: 250_000.0,
            range: 30.0,
            color: Color::srgb(0.85, 0.9, 1.0),
            ..default()
        },
        Transform::from_translation(PREVIEW_AT + Vec3::new(-2.0, 5.0, 4.0)),
    ));

    if let Some(row) = first_row {
        focus.set(row, FocusCause::Navigated);
    }
}

/// `--garage` opens it at once, for unattended looks at it.
fn open_scripted(mut script: ResMut<Script>, mut next: ResMut<NextState<Screen>>) {
    if std::mem::take(&mut script.garage) {
        next.set(Screen::Garage);
    }
}

/// A color of a palette activated from the keyboard or a gamepad, its row focused, picks it.
fn pick_swatch(
    activate: On<Activate>,
    swatches: Query<&Swatch>,
    mut steppers: Query<&mut Stepper>,
) {
    pick(activate.entity, &swatches, &mut steppers);
}

/// A click of the mouse, or a tap, on a color picks it, on the press itself and whatever else the
/// press does (since v1.0.0-rc1, the user's word "Dans le garage, on doit pouvoir selectionner les
/// couleurs à la souris"), and gives its row the focus, so the keyboard carries on from there.
fn press_swatch(
    press: On<Pointer<Press>>,
    swatches: Query<&Swatch>,
    mut steppers: Query<&mut Stepper>,
    mut focus: ResMut<InputFocus>,
) {
    if let Some(row) = pick(press.entity, &swatches, &mut steppers) {
        focus.set(row, FocusCause::Pressed);
    }
}

fn pick(entity: Entity, swatches: &Query<&Swatch>, steppers: &mut Query<&mut Stepper>) -> Option<Entity> {
    let swatch = swatches.get(entity).ok()?;
    let mut stepper = steppers.get_mut(swatch.row).ok()?;
    stepper.value = swatch.value;
    Some(swatch.row)
}

/// The color under the mouse lights up, so it reads as something to click.
fn hover_swatches(
    hovered: Query<(&Swatch, &bevy::picking::hover::Hovered)>,
    mut nodes: Query<&mut BackgroundColor, With<Swatch>>,
    swatch_entities: Query<Entity, With<Swatch>>,
) {
    for entity in &swatch_entities {
        let (Ok((swatch, hover)), Ok(mut background)) = (hovered.get(entity), nodes.get_mut(entity))
        else {
            continue;
        };
        let wanted = if hover.get() {
            swatch.color.lighter(0.12)
        } else {
            swatch.color
        };
        if background.0 != wanted {
            background.0 = wanted;
        }
    }
}

/// Keeps the choice and tells the server, then goes back to the first screen.
fn done(
    _: On<Activate>,
    rows: Query<(&Part, &Stepper)>,
    progress: Res<PlayerProgress>,
    mut preferences: ResMut<StoredPreferences>,
    mut network: ResMut<Network>,
    mut next: ResMut<NextState<Screen>>,
) {
    keep(look_of(&rows), &progress, &mut preferences, &mut network);
    next.set(Screen::Login);
}

/// Keeps what is chosen of what is unlocked: anything still locked goes back to what it was, or to
/// the first if that is locked too.
fn keep(
    chosen: CarLook,
    progress: &PlayerProgress,
    preferences: &mut StoredPreferences,
    network: &mut Network,
) {
    let xp = xp_of(progress);
    let before = rewards::allowed(preferences.get().car_look(), xp);
    let mut look = chosen;
    if !rewards::is_unlocked(Reward::Car(look.model), xp) {
        look.model = before.model;
    }
    if !rewards::is_unlocked(Reward::Trail(look.trail), xp) {
        look.trail = before.trail;
    }
    if !rewards::is_unlocked(Reward::Horn(look.horn), xp) {
        look.horn = before.horn;
    }
    if !rewards::is_unlocked(Reward::Turbo(look.turbo), xp) {
        look.turbo = before.turbo;
    }
    if preferences.get().car_look() != look {
        preferences.update(|stored| stored.look = Some(LookPreference::of(look)));
    }
    network.set_look(look);
}

/// Back, from the keyboard or a gamepad, is DONE: what is chosen is kept.
fn leave(
    input: MenuInput,
    rows: Query<(&Part, &Stepper)>,
    progress: Res<PlayerProgress>,
    mut preferences: ResMut<StoredPreferences>,
    mut network: ResMut<Network>,
    mut next: ResMut<NextState<Screen>>,
) {
    if input.back() {
        keep(look_of(&rows), &progress, &mut preferences, &mut network);
        next.set(Screen::Login);
    }
}

/// Writes what each named row holds, and whether it is locked; and the car's line. The steppers
/// step themselves (see `widgets.rs` and `navigation.rs`).
fn show_choices(
    rows: Query<(&Part, &Stepper)>,
    progress: Res<PlayerProgress>,
    mut names: Query<(&ChoiceName, &mut Text, &mut TextColor), (Without<Blurb>, Without<LockLine>)>,
    mut locks: Query<(&LockLine, &mut Text, &mut Node), (Without<Blurb>, Without<ChoiceName>)>,
    mut blurbs: Query<&mut Text, (With<Blurb>, Without<ChoiceName>, Without<LockLine>)>,
) {
    let look = look_of(&rows);
    let xp = xp_of(&progress);
    let model = CarModel::of(look.model);
    let value = |part: Part| match part {
        Part::Model => look.model,
        Part::Trail => look.trail,
        Part::Horn => look.horn,
        Part::Turbo => look.turbo,
        Part::Paint => look.paint,
        Part::Neon => look.neon,
        Part::TrailColor => look.trail_color,
    };
    let locked = |part: Part| {
        part.reward(value(part))
            .filter(|reward| !rewards::is_unlocked(*reward, xp))
    };
    for (name, mut text, mut color) in &mut names {
        let wanted = match name.0 {
            Part::Model => model.name(),
            Part::Trail => TRAILS[usize::from(look.trail)].0,
            Part::Horn => HORNS[usize::from(look.horn)],
            Part::Turbo => crate::turbo_fx::TURBOS[usize::from(look.turbo)].name,
            Part::Paint => PAINTS[usize::from(look.paint)].0,
            Part::Neon => NEONS[usize::from(look.neon)].0,
            Part::TrailColor => TRAIL_COLORS[usize::from(look.trail_color)].0,
        };
        if text.0 != wanted {
            text.0 = wanted.to_owned();
        }
        color.0 = if locked(name.0).is_some() {
            theme::TEXT_FAINT
        } else {
            theme::TEXT
        };
    }
    for (lock, mut text, mut node) in &mut locks {
        match locked(lock.0) {
            Some(reward) => {
                let wanted = format!("LOCKED · LEVEL {}", rewards::unlock_level(reward));
                if text.0 != wanted {
                    text.0 = wanted;
                }
                node.display = Display::Flex;
            }
            None => node.display = Display::None,
        }
    }
    for mut text in &mut blurbs {
        if text.0 != model.blurb() {
            text.0 = model.blurb().to_owned();
        }
    }
}

/// Rings the chosen color of every palette.
fn show_swatches(
    rows: Query<&Stepper>,
    mut swatches: Query<(&Swatch, &mut BorderColor, &mut BoxShadow, &mut UiTransform)>,
) {
    for (swatch, mut border, mut shadow, mut transform) in &mut swatches {
        let chosen = rows
            .get(swatch.row)
            .is_ok_and(|stepper| stepper.value == swatch.value);
        *border = BorderColor::all(if chosen {
            theme::TEXT
        } else {
            Color::BLACK.with_alpha(0.35)
        });
        // The chosen tile glows in its own color, a little larger than the rest.
        let wanted = if chosen {
            vec![ShadowStyle {
                color: swatch.color.with_alpha(0.8),
                x_offset: px(0),
                y_offset: px(0),
                spread_radius: px(1),
                blur_radius: px(10),
            }]
        } else {
            Vec::new()
        };
        if shadow.0 != wanted {
            shadow.0 = wanted;
        }
        let scale = Vec2::splat(if chosen { 1.18 } else { 1.0 });
        if transform.scale != scale {
            transform.scale = scale;
        }
    }
}

/// The player's level and how far into it they are, small, under the title.
fn show_level(progress: Res<PlayerProgress>, mut lines: Query<&mut Text, With<LevelLine>>) {
    let xp = xp_of(&progress);
    let wanted = format!(
        "LEVEL {}  ·  {}%",
        rewards::level(xp),
        (rewards::level_share(xp) * 100.0) as u32
    );
    for mut text in &mut lines {
        if text.0 != wanted {
            text.0.clone_from(&wanted);
        }
    }
}

/// Plays the horn as the player steps to it, never as the garage opens (since v0.1.10, the user's word
/// "pas de klaxon à l'ouverture du garage": a flag set on the first run kept every later opening
/// from being told apart, so each one honked).
fn play_horn(
    mut commands: Commands,
    rows: Query<(&Part, Ref<Stepper>), Changed<Stepper>>,
    sounds: Res<Sounds>,
) {
    for (part, stepper) in &rows {
        // A row just drawn is the garage opening, not the player stepping.
        if *part == Part::Horn
            && !stepper.is_added()
            && let Some(horn) = sounds.horns.get(usize::from(stepper.value))
        {
            sounds.play(&mut commands, horn, 0.7);
        }
    }
}

/// Fires the turbo of the car shown for a moment as the player steps to a skin (since v0.1.10), never
/// as the garage opens.
fn preview_turbo(
    rows: Query<(&Part, Ref<Stepper>), Changed<Stepper>>,
    time: Res<Time>,
    mut preview: ResMut<crate::turbo_fx::TurboPreview>,
) {
    for (part, stepper) in &rows {
        if *part == Part::Turbo && !stepper.is_added() {
            preview.until = time.elapsed_secs() + 2.2;
        }
    }
}

/// Draws the car in the look chosen, again whenever it changes, keeping its turn.
fn show_preview(
    mut commands: Commands,
    rows: Query<(&Part, &Stepper)>,
    previews: Query<(Entity, &Preview, &Transform)>,
    mut assets: ResMut<CarAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
    trail_material: Res<TrailMaterial>,
) {
    let look = look_of(&rows);
    let mut rotation = Quat::from_rotation_y(-0.6);
    for (entity, preview, transform) in &previews {
        if preview.0 == look {
            return;
        }
        rotation = transform.rotation;
        meshes.remove(&preview.1);
        commands.entity(entity).despawn();
    }
    // The trail under it, swept out behind its tail as if it had just driven a bend.
    let trail = meshes.add(
        trail::trail_geometry(
            Effect::of(look.trail),
            TRAIL_COLORS[usize::from(look.trail_color)].1,
            &trail::preview_points(),
            trail::PREVIEW_NOW,
            &trail::flat,
        )
        .into_mesh(),
    );
    let car = spawn_car(
        &mut commands,
        &mut assets,
        &mut meshes,
        look,
        None,
        false,
        (
            Preview(look, trail.clone()),
            DespawnOnExit(Screen::Garage),
            Transform::from_translation(PREVIEW_AT).with_rotation(rotation),
            Visibility::default(),
        ),
    );
    commands.entity(car).with_child((
        Mesh3d(trail),
        MeshMaterial3d(trail_material.0.clone()),
        NotShadowCaster,
    ));
}

fn turn_preview(time: Res<Time>, mut previews: Query<&mut Transform, With<Preview>>) {
    for mut transform in &mut previews {
        transform.rotate_y(TURN_RATE * time.delta_secs());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The rows step round: past the last car comes the first.
    #[test]
    fn the_choices_wrap_round() {
        let mut stepper = Stepper {
            value: CAR_MODELS - 1,
            min: 0,
            max: CAR_MODELS - 1,
            wrap: true,
        };
        stepper.step(1);
        assert_eq!(stepper.value, 0);
        stepper.step(-1);
        assert_eq!(stepper.value, CAR_MODELS - 1);
    }

    /// Every palette row holds as many colors as the wire allows.
    #[test]
    fn the_palettes_hold_every_color() {
        assert_eq!(Part::Paint.colors().len(), usize::from(CAR_PAINTS));
        assert_eq!(Part::Neon.colors().len(), usize::from(CAR_NEONS));
        assert_eq!(Part::TrailColor.colors().len(), usize::from(CAR_TRAIL_COLORS));
    }
}
