//! The beta panel (since v0.1.5, the user's word: "mets les [contrôles] dans un cadre qui devra
//! être distinctif un peu, car uniquement lié à l'état beta du jeu, il y aura donc dedans les
//! contrôles, la version, et un bouton pour envoyer feedback/déclaration de bug à
//! [l'adresse du jeu], avec un éditeur de texte simple et pratique"): a small frame outlined
//! in amber with a BETA tag on its edge, holding the controls in a race, the version (and the
//! frame rate in a race), and FEEDBACK, which opens a text editor whose text goes to the server,
//! which keeps it and mails it on (docs/feedback.md).
//!
//! It is drawn in two places: by the HUD in a lobby, with the controls, and on every other screen
//! by [`spawn_menu_panel`], the version and FEEDBACK only.

use bevy::input_focus::{FocusCause, InputFocus};
use bevy::prelude::*;
use bevy::text::{EditableText, TextCursorStyle};
use bevy::ui_widgets::Activate;
use star_racer_protocol::{ClientMessage, MAX_FEEDBACK_CHARS};

use super::safe_area::SafeInset;
use super::theme::{self, Fonts};
use super::touch::Touchscreen;
use super::widgets::{ButtonKind, button, button_of_size, panel};
use crate::net::{Connection, NetEvent, Network};
use crate::screen::Screen;

/// The panel's outline and tag: amber, the color nothing else in the interface frames with.
const BETA: Color = theme::AMBER;

/// Whether the feedback editor is open, and what it last said.
#[derive(Resource, Default)]
pub struct FeedbackDialog {
    pub open: bool,
    /// Waiting for the server's answer.
    sending: bool,
    status: Option<(String, Color)>,
}

pub struct BetaPlugin;

impl Plugin for BetaPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FeedbackDialog>()
            .add_systems(PostStartup, (spawn_menu_panel, open_on_launch))
            // Before anything reads the keyboard this frame: while the editor is open, the keys
            // are its own.
            .add_systems(PreUpdate, take_the_keyboard.after(bevy::input::InputSystems))
            .add_systems(
                Update,
                (
                    show_menu_panel,
                    show_feedback_dialog,
                    hear_server,
                    count_characters,
                    say_status,
                ),
            );
    }
}

/// The panel's frame: a dark box outlined in amber. Its children are the panel's rows; put
/// [`tag`] among them for the BETA on its edge.
pub fn frame() -> impl Bundle {
    (
        Node {
            flex_direction: FlexDirection::Column,
            // Narrow: the controls go onto two lines rather than reaching the gauges.
            max_width: px(400),
            row_gap: px(7),
            padding: UiRect::new(px(12), px(12), px(13), px(8)),
            border: px(1.5).all(),
            border_radius: BorderRadius::all(px(theme::RADIUS_SMALL)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.05, 0.035, 0.0, 0.72)),
        BorderColor::all(BETA.with_alpha(0.7)),
    )
}

/// The BETA tag, standing on the frame's top edge.
pub fn tag(fonts: &Fonts) -> impl Bundle {
    (
        Node {
            position_type: PositionType::Absolute,
            left: px(10),
            top: px(-9),
            padding: UiRect::axes(px(6), px(1)),
            border_radius: BorderRadius::all(px(3)),
            ..default()
        },
        BackgroundColor(BETA),
        Pickable::IGNORE,
        children![(
            Text::new("BETA"),
            fonts.caption(10.0),
            theme::CAPTION_SPACING,
            TextColor(theme::TEXT_ON_NEON),
        )],
    )
}

/// The panel's last row: `info` (the version, a frame rate) and the FEEDBACK button.
pub fn footer(parent: &mut ChildSpawnerCommands, fonts: &Fonts, info: impl Bundle) {
    parent
        .spawn(Node {
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceBetween,
            column_gap: px(14),
            ..default()
        })
        .with_children(|row| {
            row.spawn(info);
            row.spawn(button_of_size(fonts, ButtonKind::Quiet, "FEEDBACK / BUG", 11.5))
                .insert(Node {
                    padding: UiRect::axes(px(9), px(3)),
                    border: px(1).all(),
                    border_radius: BorderRadius::all(px(4)),
                    ..default()
                })
                .observe(open_feedback);
        });
}

/// `--feedback`: the editor open from the start, for unattended checks.
fn open_on_launch(script: Res<crate::settings::Script>, mut dialog: ResMut<FeedbackDialog>) {
    dialog.open = script.feedback;
}

fn open_feedback(_: On<Activate>, mut dialog: ResMut<FeedbackDialog>) {
    dialog.open = true;
    dialog.sending = false;
    dialog.status = None;
}

/// The panel on every screen but the lobby, whose HUD draws its own: bottom left, the version and
/// FEEDBACK.
#[derive(Component)]
struct MenuPanel;

fn spawn_menu_panel(mut commands: Commands, fonts: Res<Fonts>) {
    commands
        .spawn((
            MenuPanel,
            SafeInset,
            Pickable::IGNORE,
            GlobalZIndex(50),
            // The panel sits in a corner of this screen-wide box, which `show_menu_panel` moves to
            // where the screen shown leaves room.
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                align_items: AlignItems::FlexEnd,
                justify_content: JustifyContent::FlexStart,
                padding: UiRect::new(px(12), px(12), px(0), px(10)),
                ..default()
            },
        ))
        .with_children(|screen| {
            screen
                .spawn(frame())
                .insert(Node {
                        flex_direction: FlexDirection::Column,
                        padding: UiRect::new(px(10), px(8), px(11), px(6)),
                        border: px(1.5).all(),
                        border_radius: BorderRadius::all(px(theme::RADIUS_SMALL)),
                        ..default()
                    })
                .with_children(|panel| {
                    panel.spawn(tag(&fonts));
                    footer(
                        panel,
                        &fonts,
                        (
                            Text::new(super::version_tag()),
                            fonts.caption(10.5),
                            theme::CAPTION_SPACING,
                            TextColor(theme::TEXT_DIM),
                        ),
                    );
                });
        });
}

/// Where the panel goes on a screen, if anywhere: the bottom left corner, unless the screen has
/// something there (the user's report of 2026-11-23 that it covered the first screen; the garage's
/// DONE and, on a phone, the players online were under it too).
fn menu_panel_place(screen: Screen, touchscreen: bool) -> Option<JustifyContent> {
    match screen {
        // The lobby's HUD draws its own.
        Screen::Lobby => None,
        // The editor's panel fills the left side to the bottom; FEEDBACK is on the first screen.
        Screen::Editor => None,
        // A phone's garage and list fill the screen to its bottom edge; FEEDBACK is on the first
        // screen.
        Screen::Garage | Screen::Lobbies if touchscreen => None,
        // Its settings fill the left column down to DONE; the middle is the car, and the bottom
        // right the key hints.
        Screen::Garage => Some(JustifyContent::Center),
        _ => Some(JustifyContent::FlexStart),
    }
}

fn show_menu_panel(
    screen: Res<State<Screen>>,
    script: Res<crate::settings::Script>,
    touchscreen: Res<Touchscreen>,
    mut panels: Query<(&mut Visibility, &mut Node), With<MenuPanel>>,
) {
    let place = menu_panel_place(*screen.get(), touchscreen.0).filter(|_| !script.clean);
    for (mut visibility, mut node) in &mut panels {
        visibility.set_if_neq(if place.is_some() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
        if let Some(justify) = place
            && node.justify_content != justify
        {
            node.justify_content = justify;
        }
    }
}

#[derive(Component)]
struct FeedbackRoot;

#[derive(Component)]
struct FeedbackText;

#[derive(Component)]
struct FeedbackCount;

#[derive(Component)]
struct FeedbackStatus;

fn show_feedback_dialog(
    mut commands: Commands,
    dialog: Res<FeedbackDialog>,
    roots: Query<Entity, With<FeedbackRoot>>,
    fonts: Res<Fonts>,
    touchscreen: Res<Touchscreen>,
    mut focus: ResMut<InputFocus>,
) {
    if !dialog.is_changed() {
        return;
    }
    match (dialog.open, roots.single()) {
        (true, Err(_)) => {
            let editor = commands
                .spawn((
                    FeedbackText,
                    EditableText {
                        max_characters: Some(MAX_FEEDBACK_CHARS),
                        allow_newlines: true,
                        visible_lines: Some(if touchscreen.0 { 4.0 } else { 9.0 }),
                        ..EditableText::new("")
                    },
                    TextLayout::default(),
                    fonts.text(16.0, 500),
                    TextColor(theme::TEXT),
                    TextCursorStyle {
                        color: theme::NEON,
                        selection_color: theme::NEON_DIM,
                        unfocused_selection_color: Color::NONE,
                        selected_text_color: None,
                    },
                    Node {
                        width: percent(100),
                        padding: UiRect::axes(px(12), px(9)),
                        border: px(1.5).all(),
                        border_radius: BorderRadius::all(px(theme::RADIUS_SMALL)),
                        ..default()
                    },
                    // Opaque: what is behind the dialog must not show through the text.
                    BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.85)),
                    BorderColor::all(BETA.with_alpha(0.5)),
                ))
                .id();
            let cancel = commands
                .spawn(button(&fonts, ButtonKind::Secondary, "CANCEL"))
                .observe(|_: On<Activate>, mut dialog: ResMut<FeedbackDialog>| {
                    dialog.open = false;
                })
                .id();
            let send = commands
                .spawn(button(&fonts, ButtonKind::Primary, "SEND"))
                .observe(send_feedback)
                .id();
            let mut card_entity = None;
            commands
                .spawn((
                    FeedbackRoot,
                    GlobalZIndex(200),
                    Node {
                        position_type: PositionType::Absolute,
                        width: percent(100),
                        height: percent(100),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(theme::BACKDROP),
                ))
                .with_children(|root| {
                    root.spawn((
                        Node {
                            width: px(if touchscreen.0 { 560.0 } else { 640.0 }),
                            flex_direction: FlexDirection::Column,
                            padding: px(22).all(),
                            row_gap: px(10),
                            border: px(1.5).all(),
                            border_radius: BorderRadius::all(px(theme::RADIUS)),
                            ..default()
                        },
                        panel(),
                    ))
                    .insert(BorderColor::all(BETA.with_alpha(0.7)))
                    .with_children(|card| {
                        card.spawn(tag(&fonts));
                        card.spawn((
                            Text::new("FEEDBACK  ·  BUG REPORT"),
                            fonts.display(if touchscreen.0 { 24.0 } else { 30.0 }),
                            TextColor(theme::TEXT),
                        ));
                        card.spawn((
                            Text::new(
                                "What you liked, what you didn't, what went wrong and how. It goes \
                                 to the team with the game's version.",
                            ),
                            fonts.text(14.0, 400),
                            TextColor(theme::TEXT_DIM),
                        ));
                        card_entity = Some(card.target_entity());
                        // The editor goes here, after the two lines above (see below).
                        card.spawn(Node::default());
                        card.spawn(Node {
                            justify_content: JustifyContent::SpaceBetween,
                            align_items: AlignItems::Center,
                            column_gap: px(12),
                            ..default()
                        })
                        .with_children(|row| {
                            row.spawn((
                                FeedbackStatus,
                                Text::default(),
                                fonts.text(13.5, 500),
                                TextColor(theme::TEXT_DIM),
                                Node {
                                    flex_grow: 1.0,
                                    ..default()
                                },
                            ));
                            row.spawn((
                                FeedbackCount,
                                Text::new(format!("0 / {MAX_FEEDBACK_CHARS}")),
                                fonts.caption(11.0),
                                TextColor(theme::TEXT_FAINT),
                            ));
                        });
                        card.spawn((
                            Text::new("Ctrl+Enter sends, Escape closes."),
                            fonts.caption(10.5),
                            theme::CAPTION_SPACING,
                            TextColor(theme::TEXT_FAINT),
                        ));
                        card.spawn(Node {
                            justify_content: JustifyContent::FlexEnd,
                            column_gap: px(12),
                            margin: UiRect::top(px(6)),
                            ..default()
                        })
                        .add_children(&[cancel, send]);
                    });
                });
            if let Some(card) = card_entity {
                commands.entity(card).insert_children(3, &[editor]);
            }
            focus.set(editor, FocusCause::Navigated);
        }
        (false, Ok(root)) => {
            commands.entity(root).despawn();
            focus.clear();
        }
        _ => {}
    }
}

/// What the client says about where feedback comes from, beside the player's text.
fn context(screen: Screen, touchscreen: bool) -> String {
    let platform = if cfg!(target_arch = "wasm32") {
        "web"
    } else {
        "native"
    };
    let device = if touchscreen { ", touch" } else { "" };
    format!(
        "{}  ·  {platform}{device}  ·  screen {screen:?}",
        super::version()
    )
}

fn send_feedback(
    _: On<Activate>,
    mut dialog: ResMut<FeedbackDialog>,
    texts: Query<&EditableText, With<FeedbackText>>,
    network: Res<Network>,
    connection: Res<Connection>,
    screen: Res<State<Screen>>,
    touchscreen: Res<Touchscreen>,
) {
    let Ok(editable) = texts.single() else {
        return;
    };
    submit(
        &mut dialog,
        &editable.value().to_string(),
        &network,
        &connection,
        context(*screen.get(), touchscreen.0),
    );
}

fn submit(
    dialog: &mut FeedbackDialog,
    text: &str,
    network: &Network,
    connection: &Connection,
    context: String,
) {
    if dialog.sending {
        return;
    }
    let text = text.trim();
    if text.is_empty() {
        dialog.status = Some(("Write something first.".into(), theme::AMBER));
        return;
    }
    if !matches!(connection, Connection::Online { .. }) {
        dialog.status = Some((
            "Not connected to the server: connect first, then send.".into(),
            theme::RED,
        ));
        return;
    }
    network.send(ClientMessage::Feedback {
        text: text.to_owned(),
        context,
    });
    dialog.sending = true;
    dialog.status = Some(("Sending…".into(), theme::TEXT_DIM));
}

/// The server's answer: thanks and close, or why not.
fn hear_server(
    mut events: MessageReader<NetEvent>,
    mut dialog: ResMut<FeedbackDialog>,
    mut texts: Query<&mut EditableText, With<FeedbackText>>,
) {
    for event in events.read() {
        let NetEvent::FeedbackReceived(kept) = event else {
            continue;
        };
        if !dialog.sending {
            continue;
        }
        dialog.sending = false;
        if *kept {
            dialog.status = Some(("Sent. Thank you!".into(), theme::GREEN));
            for mut text in &mut texts {
                super::widgets::set_text(&mut text, "");
            }
        } else {
            dialog.status = Some((
                "Not sent: too long, or too soon after the last one. Try again in a moment."
                    .into(),
                theme::RED,
            ));
        }
    }
}

/// While the editor is open the keyboard is its own: Escape closes it, Ctrl+Enter sends, and
/// nothing else in the game hears a key (the car does not drive, Escape opens no menu). The editor
/// reads the keyboard's own events, which this leaves alone.
fn take_the_keyboard(
    mut keyboard: ResMut<ButtonInput<KeyCode>>,
    mut dialog: ResMut<FeedbackDialog>,
    texts: Query<&EditableText, With<FeedbackText>>,
    network: Res<Network>,
    connection: Res<Connection>,
    screen: Res<State<Screen>>,
    touchscreen: Res<Touchscreen>,
) {
    if !dialog.open {
        return;
    }
    let control = keyboard.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]);
    if keyboard.just_pressed(KeyCode::Escape) {
        dialog.open = false;
    } else if control && keyboard.just_pressed(KeyCode::Enter) {
        if let Ok(editable) = texts.single() {
            let text = editable.value().to_string();
            submit(
                &mut dialog,
                &text,
                &network,
                &connection,
                context(*screen.get(), touchscreen.0),
            );
        }
    }
    keyboard.reset_all();
}

fn count_characters(
    texts: Query<&EditableText, (With<FeedbackText>, Changed<EditableText>)>,
    mut counts: Query<(&mut Text, &mut TextColor), With<FeedbackCount>>,
) {
    for editable in &texts {
        let count = editable.value().to_string().chars().count();
        for (mut text, mut color) in &mut counts {
            text.0 = format!("{count} / {MAX_FEEDBACK_CHARS}");
            color.0 = if count * 10 >= MAX_FEEDBACK_CHARS * 9 {
                theme::AMBER
            } else {
                theme::TEXT_FAINT
            };
        }
    }
}

fn say_status(
    dialog: Res<FeedbackDialog>,
    mut statuses: Query<(&mut Text, &mut TextColor), With<FeedbackStatus>>,
) {
    if !dialog.is_changed() {
        return;
    }
    for (mut text, mut color) in &mut statuses {
        let (status, status_color) = dialog
            .status
            .clone()
            .unwrap_or_else(|| (String::new(), theme::TEXT_DIM));
        text.0 = status;
        color.0 = status_color;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feedback_says_where_it_comes_from() {
        let said = context(Screen::Lobby, true);
        assert!(said.starts_with(&super::super::version()), "{said}");
        assert!(said.contains("touch") && said.contains("Lobby"), "{said}");
    }

    #[test]
    fn the_panel_keeps_out_of_what_a_screen_holds() {
        assert_eq!(menu_panel_place(Screen::Login, false), Some(JustifyContent::FlexStart));
        assert_eq!(menu_panel_place(Screen::Garage, false), Some(JustifyContent::Center));
        assert_eq!(menu_panel_place(Screen::Garage, true), None);
        assert_eq!(menu_panel_place(Screen::Lobbies, false), Some(JustifyContent::FlexStart));
        assert_eq!(menu_panel_place(Screen::Lobbies, true), None);
        assert_eq!(menu_panel_place(Screen::Lobby, false), None);
    }
}
