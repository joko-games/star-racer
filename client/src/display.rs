//! How the game takes the screen (since v0.1.5, the user's word: "Offre de la customisation dans
//! les settings graphiques, comme le mode plein écran/fullscreen/fenêtré, adapté dans le cas du
//! web également"). Natively three modes, kept with the player's choices and applied at once:
//! WINDOWED, BORDERLESS (a window the size of the screen, which switching away from leaves alone)
//! and FULL SCREEN (the screen given over to the game). In a browser, two: the page as it is, or
//! the browser's full screen; that one is asked of the browser from the button's press, the only
//! time it allows it, and never kept, since a page cannot open full screen by itself.

use bevy::prelude::*;
use bevy::window::{MonitorSelection, PrimaryWindow, VideoModeSelection, WindowMode};
use serde::{Deserialize, Serialize};

use crate::preferences::StoredPreferences;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum DisplayMode {
    #[default]
    Windowed,
    Borderless,
    FullScreen,
}

impl DisplayMode {
    /// The next mode, as the settings step through them: a browser has two.
    pub fn next(self) -> Self {
        match (self, cfg!(target_arch = "wasm32")) {
            (DisplayMode::Windowed, true) => DisplayMode::FullScreen,
            (_, true) => DisplayMode::Windowed,
            (DisplayMode::Windowed, false) => DisplayMode::Borderless,
            (DisplayMode::Borderless, false) => DisplayMode::FullScreen,
            (DisplayMode::FullScreen, false) => DisplayMode::Windowed,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            DisplayMode::Windowed => "WINDOWED",
            DisplayMode::Borderless => "BORDERLESS",
            DisplayMode::FullScreen => "FULL SCREEN",
        }
    }

    fn window_mode(self) -> WindowMode {
        match self {
            DisplayMode::Windowed => WindowMode::Windowed,
            DisplayMode::Borderless => WindowMode::BorderlessFullscreen(MonitorSelection::Current),
            DisplayMode::FullScreen => {
                WindowMode::Fullscreen(MonitorSelection::Current, VideoModeSelection::Current)
            }
        }
    }
}

/// `fixed`: a window size was asked for on the command line (captures, films), which no stored
/// mode overrides.
pub struct DisplayPlugin {
    pub fixed: bool,
}

impl Plugin for DisplayPlugin {
    fn build(&self, app: &mut App) {
        if !self.fixed {
            app.add_systems(Update, apply_display_mode);
        }
    }
}

/// The mode the player chose, on the window. Natively from the first frame; in a browser only once
/// changed from the settings (see the module's comment).
fn apply_display_mode(
    preferences: Res<StoredPreferences>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
    mut applied: Local<Option<DisplayMode>>,
) {
    let wanted = preferences.get().display;
    if *applied == Some(wanted) {
        return;
    }
    let first = applied.is_none();
    *applied = Some(wanted);
    if cfg!(target_arch = "wasm32") {
        if !first {
            browser_full_screen(wanted == DisplayMode::FullScreen);
        }
        return;
    }
    if let Ok(mut window) = windows.single_mut() {
        window.mode = wanted.window_mode();
        // Windowed, the game opens maximized (since v1.0.0-rc16): Bevy's window of 1280 by 720
        // and its title bar ran under the taskbar of a screen 768 high, the user's 1366 by 768.
        if first && wanted == DisplayMode::Windowed {
            window.set_maximized(true);
        }
    }
}

/// Asks the browser for its full screen, or to leave it.
fn browser_full_screen(full: bool) {
    #[cfg(target_arch = "wasm32")]
    if let Some(document) = web_sys::window().and_then(|window| window.document()) {
        if full {
            if let Some(page) = document.document_element() {
                let _ = page.request_fullscreen();
            }
        } else if document.fullscreen_element().is_some() {
            document.exit_fullscreen();
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = full;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_modes_come_round() {
        let mut mode = DisplayMode::Windowed;
        let mut seen = vec![mode];
        for _ in 0..3 {
            mode = mode.next();
            seen.push(mode);
        }
        assert_eq!(seen.last(), Some(&DisplayMode::Windowed));
        assert!(seen.contains(&DisplayMode::FullScreen));
    }
}
