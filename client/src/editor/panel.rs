//! The editor's panel, down the left of the screen: the pieces of the road in order, what can be
//! added, the numbers of the piece selected, what the editor has to say, and TRY, NEW and BACK.
//!
//! It is drawn again whole whenever something it shows changes ([`refresh`]): a handful of rows,
//! nothing a frame notices. Every button carries the [`Action`] it does, done by one observer.

use bevy::input::mouse::{AccumulatedMouseScroll, MouseScrollUnit};
use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use bevy::ui_widgets::Activate;
use bevy::window::PrimaryWindow;

use super::model::{Draft, Field, Kind, New};
use super::{Editor, try_draft};
use crate::lobby::Tracks;
use crate::net::Network;
use crate::screen::{AfterLobby, Screen};
use crate::ui::theme::{self, Fonts};
use crate::ui::widgets::{ButtonKind, button_of_size, panel};

/// How wide the panel is, in pixels: the map is the rest of the window.
pub const WIDTH: f32 = 380.0;

/// What a button of the panel does.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub(super) enum Action {
    Select(usize),
    Add(New),
    Delete,
    Step(Field, f32),
    ClosingRadius(f32),
    Undo,
    Redo,
    Try,
    New,
    Back,
}

/// The panel's root, drawn again whole when something it shows changes.
#[derive(Component)]
pub(super) struct Root;

/// The list of pieces, which the wheel scrolls.
#[derive(Component)]
pub(super) struct PieceList;

pub(super) fn observe(app: &mut App) {
    app.add_observer(act).add_systems(
        Update,
        scroll_list.run_if(in_state(Screen::Editor)),
    );
}

/// On entering: the panel's root, filled by [`refresh`].
pub(super) fn spawn(mut commands: Commands, mut editor: ResMut<Editor>) {
    commands.spawn((
        Root,
        DespawnOnExit(Screen::Editor),
        Node {
            position_type: PositionType::Absolute,
            left: px(0),
            top: px(0),
            width: px(WIDTH),
            height: percent(100),
            flex_direction: FlexDirection::Column,
            padding: UiRect::new(px(16), px(16), px(14), px(14)),
            row_gap: px(8),
            border: UiRect::right(px(1)),
            ..default()
        },
        panel(),
    ));
    editor.panel_dirty = true;
}

/// Draws the panel again when the editor has something new to show.
pub(super) fn refresh(
    mut commands: Commands,
    mut editor: ResMut<Editor>,
    roots: Query<Entity, With<Root>>,
    fonts: Res<Fonts>,
) {
    if !editor.panel_dirty {
        return;
    }
    let Ok(root) = roots.single() else {
        return;
    };
    editor.panel_dirty = false;
    commands.entity(root).despawn_children();
    let editor = &*editor;
    commands.entity(root).with_children(|panel| {
        panel.spawn((
            Text::new("EDITOR"),
            fonts.display(30.0),
            TextColor(theme::TEXT),
            TextShadow {
                offset: Vec2::new(3.0, 3.0),
                color: theme::NEON.with_alpha(0.8),
            },
        ));
        panel.spawn((
            Text::new(
                "Lay the road piece by piece: the editor closes it back to the line. \
                 Raise a piece to pass over the road.",
            ),
            fonts.text(12.5, 400),
            TextColor(theme::TEXT_DIM),
        ));

        caption(panel, &fonts, "THE ROAD");
        panel
            .spawn((
                PieceList,
                ScrollPosition::default(),
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: px(3),
                    max_height: px(230),
                    overflow: Overflow::scroll_y(),
                    flex_shrink: 0.0,
                    ..default()
                },
            ))
            .with_children(|list| {
                for (index, piece) in editor.draft.pieces.iter().enumerate() {
                    let selected = editor.selected == Some(index);
                    let label = format!("{:>2}  {}", index + 1, summary(piece));
                    let kind = if selected {
                        ButtonKind::Secondary
                    } else {
                        ButtonKind::Quiet
                    };
                    list.spawn((Action::Select(index), button_of_size(&fonts, kind, label, 13.0)))
                        .insert(row_node());
                }
            });
        panel
            .spawn(Node {
                column_gap: px(6),
                ..default()
            })
            .with_children(|row| {
                for (new, label) in [
                    (New::Straight, "+ STRAIGHT"),
                    (New::Left, "+ LEFT"),
                    (New::Right, "+ RIGHT"),
                ] {
                    small(row, &fonts, Action::Add(new), label, ButtonKind::Secondary, None);
                }
            });

        if let Some(index) = editor.selected
            && let Some(piece) = editor.draft.pieces.get(index)
        {
            let name = match piece.kind {
                Kind::Straight => "STRAIGHT".to_owned(),
                Kind::Turn if piece.angle >= 0.0 => "TURN LEFT".to_owned(),
                Kind::Turn => "TURN RIGHT".to_owned(),
            };
            caption(panel, &fonts, &format!("PIECE {} · {name}", index + 1));
            for &field in Field::of(piece.kind) {
                stepper_row(panel, &fonts, field.name(), &value_text(piece.get(field), field), |delta| {
                    Action::Step(field, delta)
                });
            }
        }

        panel
            .spawn(Node {
                column_gap: px(6),
                ..default()
            })
            .with_children(|row| {
                let none = editor.selected.is_none();
                small(row, &fonts, Action::Delete, "DELETE", ButtonKind::Quiet, Some(none));
                small(row, &fonts, Action::Undo, "UNDO", ButtonKind::Quiet, Some(!editor.can_undo()));
                small(row, &fonts, Action::Redo, "REDO", ButtonKind::Quiet, Some(!editor.can_redo()));
            });

        caption(panel, &fonts, "THE WAY BACK");
        stepper_row(
            panel,
            &fonts,
            "RADIUS",
            &format!("{:.0} m", editor.draft.closing_radius),
            Action::ClosingRadius,
        );

        // What it comes to, or why it does not build.
        let (status, color) = match &editor.built {
            Ok(built) => (
                format!(
                    "{:.2} km round, {} piece{} added to close it.",
                    built.track.length() / 1000.0,
                    built.closed.closing,
                    if built.closed.closing == 1 { "" } else { "s" }
                ),
                theme::GREEN,
            ),
            Err(why) => (format!("Does not build: {why}."), theme::RED),
        };
        panel.spawn((Text::new(status), fonts.text(12.5, 500), TextColor(color)));
        if let Some((message, bad)) = &editor.message {
            let color = if *bad { theme::RED } else { theme::TEXT_DIM };
            panel.spawn((Text::new(message.clone()), fonts.text(12.5, 500), TextColor(color)));
        }

        // Pushed to the bottom.
        panel.spawn(Node {
            flex_grow: 1.0,
            ..default()
        });
        panel
            .spawn(Node {
                column_gap: px(8),
                ..default()
            })
            .with_children(|row| {
                let mut try_button = row.spawn((
                    Action::Try,
                    button_of_size(&fonts, ButtonKind::Primary, "TRY", 16.0),
                ));
                try_button.insert(Node {
                    flex_grow: 1.0,
                    ..row_node()
                });
                if editor.built.is_err() {
                    try_button.insert(InteractionDisabled);
                }
                small(row, &fonts, Action::New, "NEW", ButtonKind::Quiet, None);
                small(row, &fonts, Action::Back, "BACK", ButtonKind::Quiet, None);
            });
        panel.spawn((
            Text::new("Click a piece on the map · wheel zooms · right button drags · Ctrl+Z undoes"),
            fonts.text(11.0, 400),
            TextColor(theme::TEXT_FAINT),
        ));
    });
}

/// How a piece reads in the list.
fn summary(piece: &super::model::Piece) -> String {
    let shape = match piece.kind {
        Kind::Straight => format!("STRAIGHT {:.0} m", piece.length),
        Kind::Turn => format!(
            "{} {:.0}° r{:.0}",
            if piece.angle >= 0.0 { "LEFT" } else { "RIGHT" },
            piece.angle.abs(),
            piece.radius
        ),
    };
    let mut extra = format!("  ·  {:.0} m wide", piece.width);
    if piece.height != 0.0 {
        extra += &format!("  ·  {:+.0} m", piece.height);
    }
    shape + &extra
}

fn value_text(value: f32, field: Field) -> String {
    match field {
        Field::Angle if value >= 0.0 => format!("{value:.0}{} left", field.unit()),
        Field::Angle => format!("{:.0}{} right", -value, field.unit()),
        _ => format!("{value:.0}{}", field.unit()),
    }
}

fn caption(panel: &mut ChildSpawnerCommands, fonts: &Fonts, text: &str) {
    panel.spawn((
        Text::new(text),
        fonts.caption(12.5),
        theme::CAPTION_SPACING,
        TextColor(theme::NEON),
        Node {
            margin: UiRect::top(px(4)),
            ..default()
        },
    ));
}

/// A name, then «, ‹, the value, › and »: ten steps and one either way.
fn stepper_row(
    panel: &mut ChildSpawnerCommands,
    fonts: &Fonts,
    name: &str,
    value: &str,
    action: impl Fn(f32) -> Action,
) {
    panel
        .spawn(Node {
            align_items: AlignItems::Center,
            column_gap: px(4),
            ..default()
        })
        .with_children(|row| {
            row.spawn((
                Text::new(name),
                fonts.caption(12.0),
                theme::CAPTION_SPACING,
                TextColor(theme::TEXT_DIM),
                Node {
                    width: px(82),
                    flex_shrink: 0.0,
                    ..default()
                },
            ));
            small(row, fonts, action(-10.0), "«", ButtonKind::Quiet, None);
            small(row, fonts, action(-1.0), "‹", ButtonKind::Quiet, None);
            row.spawn((
                Text::new(value),
                fonts.text(14.0, 700),
                TextColor(theme::TEXT),
                TextLayout::justify(Justify::Center),
                Node {
                    flex_grow: 1.0,
                    ..default()
                },
            ));
            small(row, fonts, action(1.0), "›", ButtonKind::Quiet, None);
            small(row, fonts, action(10.0), "»", ButtonKind::Quiet, None);
        });
}

/// A small button doing `action`, greyed when `disabled` says so.
fn small(
    row: &mut ChildSpawnerCommands,
    fonts: &Fonts,
    action: Action,
    label: &str,
    kind: ButtonKind,
    disabled: Option<bool>,
) {
    let mut button = row.spawn((action, button_of_size(fonts, kind, label, 13.0)));
    button.insert(Node {
        padding: UiRect::axes(px(9), px(5)),
        ..row_node()
    });
    if disabled == Some(true) {
        button.insert(InteractionDisabled);
    }
}

fn row_node() -> Node {
    Node {
        padding: UiRect::axes(px(10), px(5)),
        justify_content: JustifyContent::FlexStart,
        align_items: AlignItems::Center,
        border: px(1.5).all(),
        border_radius: BorderRadius::all(px(theme::RADIUS_SMALL)),
        ..default()
    }
}

/// Removes the piece selected, keeping one at least.
pub(super) fn delete_selected(editor: &mut Editor) {
    let Some(index) = editor.selected else {
        return;
    };
    if editor.draft.pieces.len() <= 1 {
        editor.say("A circuit needs one piece of road at least.", true);
        return;
    }
    editor.draft.pieces.remove(index);
    editor.selected = Some(index.saturating_sub(1).min(editor.draft.pieces.len() - 1));
    editor.changed();
}

/// Does what the button pressed says.
#[allow(clippy::too_many_arguments)]
fn act(
    activate: On<Activate>,
    actions: Query<&Action>,
    editor: Option<ResMut<Editor>>,
    network: Res<Network>,
    mut tracks: ResMut<Tracks>,
    mut images: ResMut<Assets<Image>>,
    mut after: ResMut<AfterLobby>,
    mut next: ResMut<NextState<Screen>>,
) {
    let (Ok(&action), Some(mut editor)) = (actions.get(activate.entity), editor) else {
        return;
    };
    match action {
        Action::Select(index) => {
            editor.selected = Some(index);
            editor.panel_dirty = true;
        }
        Action::Add(new) => {
            let selected = editor.selected;
            let at = editor.draft.add(new, selected);
            editor.selected = Some(at);
            editor.changed();
        }
        Action::Delete => delete_selected(&mut editor),
        Action::Step(field, steps) => {
            if let Some(index) = editor.selected
                && let Some(piece) = editor.draft.pieces.get_mut(index)
            {
                piece.step(field, steps);
                editor.changed();
            }
        }
        Action::ClosingRadius(steps) => {
            editor.draft.closing_radius = (editor.draft.closing_radius + 5.0 * steps).clamp(30.0, 500.0);
            editor.changed();
        }
        Action::Undo => editor.undo(),
        Action::Redo => editor.redo(),
        Action::Try => try_draft(&mut editor, &network, &mut tracks, &mut images, &mut after),
        Action::New => {
            editor.draft = Draft::starter();
            editor.selected = Some(editor.draft.pieces.len() - 1);
            editor.reframe = true;
            editor.changed();
        }
        Action::Back => next.set(Screen::Login),
    }
}

/// The wheel over the panel scrolls the list of pieces.
fn scroll_list(
    scroll: Res<AccumulatedMouseScroll>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut lists: Query<&mut ScrollPosition, With<PieceList>>,
) {
    let over_panel = windows
        .single()
        .ok()
        .and_then(Window::cursor_position)
        .is_some_and(|cursor| cursor.x <= WIDTH);
    if !over_panel || scroll.delta.y == 0.0 {
        return;
    }
    let pixels = match scroll.unit {
        MouseScrollUnit::Line => scroll.delta.y * 30.0,
        MouseScrollUnit::Pixel => scroll.delta.y,
    };
    for mut position in &mut lists {
        position.y = (position.y - pixels).max(0.0);
    }
}
