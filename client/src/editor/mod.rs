//! The level editor, made again from nothing (since v1.0.0-rc11, the user's word on 2026-11-25:
//! "Faire le début de l'éditeur"; asked, they chose it in the game, a first version that lays a
//! circuit, tries it and keeps it locally, publishing waiting for the accounts on the site).
//!
//! A screen of its own, EDITOR on the first screen (not on a phone): the circuit as it will be
//! raced, built by the race's own code and seen from above ([`map`]), and a panel to lay its road
//! piece by piece ([`panel`]): straights and turns, each with its own width, banking and the height
//! the road reaches at its end, so the road may climb over itself. The editor closes the road back
//! to the line by itself (`editing::closing`). TRY races it alone in a lobby nobody else sees, and
//! leaving that lobby comes back here. The draft is kept beside the player's other choices, so it
//! survives a restart. See docs/editor.md.

mod map;
pub mod model;
mod panel;

use std::sync::Arc;

use bevy::prelude::*;
use star_racer_protocol::ClientMessage;
use star_racer_sim::track::Track;

use self::model::{Closed, Draft};
use crate::lobby::{Tracks, join_error_text};
use crate::net::{NetEvent, Network};
use crate::preferences::StoredPreferences;
use crate::screen::{AfterLobby, Screen};

/// Where the draft is kept, beside the player's other choices.
const SLOT: &str = "editor.ron";

/// How many drafts back an undo can go.
const HISTORY: usize = 100;

pub struct EditorPlugin;

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(Screen::Editor), (open, panel::spawn).chain())
            .add_systems(OnExit(Screen::Editor), close)
            // The circuit tried is nobody's to pick anywhere else.
            .add_systems(OnEnter(Screen::Login), forget_try)
            .add_systems(OnEnter(Screen::Lobbies), forget_try)
            .add_systems(
                Update,
                (
                    hear_server,
                    shortcuts,
                    leave_on_back,
                    scripted_try,
                    panel::refresh,
                    map::rebuild,
                    map::steer_camera,
                    map::pick,
                )
                    .chain()
                    .run_if(in_state(Screen::Editor)),
            );
        panel::observe(app);
    }
}

/// The editor's state: the draft, what it builds into, and how it is being looked at.
#[derive(Resource)]
pub struct Editor {
    pub draft: Draft,
    /// The piece selected: what the panel shows the numbers of, and where a new piece goes after.
    pub selected: Option<usize>,
    /// The circuit closed and built as the server will build it, or why it cannot be.
    pub built: Result<Built, String>,
    /// What the editor last has to say, and whether it is bad news.
    pub message: Option<(String, bool)>,
    /// Something changed that the panel shows, or that the map shows.
    pub panel_dirty: bool,
    pub scene_dirty: bool,
    /// The camera should frame the whole circuit again.
    pub reframe: bool,
    history: Vec<Draft>,
    future: Vec<Draft>,
    /// The draft as it stood after the last change, what an undo goes back past.
    last: Draft,
}

/// A draft closed into a circuit that builds.
pub struct Built {
    pub closed: Closed,
    pub track: Arc<Track>,
}

impl Editor {
    fn new(draft: Draft) -> Self {
        let mut editor = Self {
            last: draft.clone(),
            history: Vec::new(),
            future: Vec::new(),
            selected: draft.pieces.len().checked_sub(1),
            draft,
            built: Err(String::new()),
            message: None,
            panel_dirty: true,
            scene_dirty: true,
            reframe: true,
        };
        editor.build();
        editor
    }

    /// Closes the draft and builds it, as the server will.
    fn build(&mut self) {
        self.built = self.draft.description().and_then(|closed| {
            Track::build(&closed.description)
                .map(|track| Built {
                    closed,
                    track: Arc::new(track),
                })
                .map_err(|error| error.to_string())
        });
    }

    /// The draft changed: it is built again, the panel and the map redrawn, and what it was kept
    /// for an undo.
    pub fn changed(&mut self) {
        if self.draft != self.last {
            let before = std::mem::replace(&mut self.last, self.draft.clone());
            self.history.push(before);
            if self.history.len() > HISTORY {
                self.history.remove(0);
            }
            self.future.clear();
        }
        self.selected = self
            .selected
            .map(|index| index.min(self.draft.pieces.len().saturating_sub(1)))
            .filter(|_| !self.draft.pieces.is_empty());
        self.build();
        self.message = None;
        self.panel_dirty = true;
        self.scene_dirty = true;
    }

    pub fn undo(&mut self) {
        if let Some(before) = self.history.pop() {
            self.future.push(std::mem::replace(&mut self.draft, before));
            self.after_undo("Undone.");
        }
    }

    pub fn redo(&mut self) {
        if let Some(after) = self.future.pop() {
            self.history.push(std::mem::replace(&mut self.draft, after));
            self.after_undo("Redone.");
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.history.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.future.is_empty()
    }

    fn after_undo(&mut self, said: &str) {
        self.last = self.draft.clone();
        self.selected = self
            .selected
            .filter(|index| *index < self.draft.pieces.len())
            .or(self.draft.pieces.len().checked_sub(1));
        self.build();
        self.say(said, false);
        self.scene_dirty = true;
    }

    pub fn say(&mut self, text: impl Into<String>, bad: bool) {
        self.message = Some((text.into(), bad));
        self.panel_dirty = true;
    }

    fn keep(&self, preferences: &StoredPreferences) {
        match ron::ser::to_string_pretty(&self.draft, ron::ser::PrettyConfig::default()) {
            Ok(text) => {
                if let Err(error) = preferences.store().write(SLOT, &text) {
                    warn!("cannot keep the editor's draft: {error:#}");
                }
            }
            Err(error) => warn!("cannot write down the editor's draft: {error}"),
        }
    }
}

/// On entering: the editor as it was left this run (coming back from a try), else the draft kept
/// from the last run, else a first circuit.
fn open(mut commands: Commands, existing: Option<ResMut<Editor>>, preferences: Res<StoredPreferences>) {
    if let Some(mut editor) = existing {
        editor.panel_dirty = true;
        editor.scene_dirty = true;
        return;
    }
    let kept = preferences
        .store()
        .read(SLOT)
        .and_then(|text| ron::from_str::<Draft>(&text).ok());
    commands.insert_resource(Editor::new(kept.unwrap_or_else(Draft::starter)));
}

fn close(
    mut commands: Commands,
    editor: Option<Res<Editor>>,
    preferences: Res<StoredPreferences>,
    scenes: Query<Entity, With<map::EditorScene>>,
) {
    if let Some(editor) = editor {
        editor.keep(&preferences);
    }
    for scene in &scenes {
        commands.entity(scene).despawn();
    }
}

/// What the server answers the editor: a try it refused.
fn hear_server(mut events: MessageReader<NetEvent>, mut editor: ResMut<Editor>) {
    for event in events.read() {
        if let NetEvent::JoinFailed(error) = event {
            editor.say(join_error_text(*error), true);
        }
    }
}

/// Escape, or B, goes back to the first screen, the draft kept.
fn leave_on_back(input: crate::ui::navigation::MenuInput, mut next: ResMut<NextState<Screen>>) {
    if input.back() {
        next.set(Screen::Login);
    }
}

/// TRY: the circuit is kept for the lobby to draw, and the server asked for a lobby racing it;
/// leaving that lobby comes back here.
pub fn try_draft(
    editor: &mut Editor,
    network: &Network,
    tracks: &mut Tracks,
    images: &mut Assets<Image>,
    after: &mut AfterLobby,
) {
    let Ok(built) = &editor.built else {
        editor.say("The circuit does not build yet: see why below.", true);
        return;
    };
    let description = built.closed.description.clone();
    tracks.store_try(&description, Track::clone(&built.track), images);
    after.0 = Screen::Editor;
    network.send(ClientMessage::TryTrack(description));
    editor.say("Opening a lobby to try it…", false);
}

fn forget_try(mut tracks: ResMut<Tracks>) {
    tracks.0.remove(star_racer_protocol::TRY_TRACK);
}

/// `--editor-try`: TRY pressed once, as soon as the draft builds.
fn scripted_try(
    mut script: ResMut<crate::settings::Script>,
    mut editor: ResMut<Editor>,
    network: Res<Network>,
    mut tracks: ResMut<Tracks>,
    mut images: ResMut<Assets<Image>>,
    mut after: ResMut<AfterLobby>,
) {
    if script.editor_try && editor.built.is_ok() {
        script.editor_try = false;
        try_draft(&mut editor, &network, &mut tracks, &mut images, &mut after);
    }
}

/// Ctrl+Z undoes, Ctrl+Y or Ctrl+Shift+Z redoes, Delete removes the piece selected.
fn shortcuts(keys: Res<ButtonInput<KeyCode>>, mut editor: ResMut<Editor>) {
    let control = keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]);
    let shift = keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
    if control && keys.just_pressed(KeyCode::KeyZ) {
        if shift {
            editor.redo();
        } else {
            editor.undo();
        }
    } else if control && keys.just_pressed(KeyCode::KeyY) {
        editor.redo();
    } else if keys.just_pressed(KeyCode::Delete) {
        panel::delete_selected(&mut editor);
    }
}

#[cfg(test)]
mod tests {
    use super::model::New;
    use super::*;

    /// A change can be undone and redone, and a new change forgets what was undone.
    #[test]
    fn changes_are_undone_and_redone() {
        let mut editor = Editor::new(Draft::starter());
        let start = editor.draft.clone();
        assert!(!editor.can_undo());
        editor.draft.add(New::Straight, None);
        editor.changed();
        let one = editor.draft.clone();
        assert!(editor.can_undo());
        editor.undo();
        assert_eq!(editor.draft, start);
        assert!(editor.can_redo());
        editor.redo();
        assert_eq!(editor.draft, one);
        editor.undo();
        editor.draft.pieces[0].width += 2.0;
        editor.changed();
        assert!(!editor.can_redo(), "a new change forgets what was undone");
        editor.undo();
        assert_eq!(editor.draft, start);
    }
}
