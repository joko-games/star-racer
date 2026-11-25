//! The game's interface: the login screen, the lobby browser and its creation dialog, the race HUD,
//! the settings, and what they are built from.

mod back;
mod backdrop;
pub mod beta;
mod create_lobby;
mod garage;
mod gauges;
pub mod hud;
pub mod leaderboard;
mod lobbies;
mod login;
pub mod navigation;
mod sound_graphics;
mod player_card;
pub mod safe_area;
mod settings;
pub mod theme;
pub mod thumbnail;
pub mod touch;
pub mod widgets;

use bevy::prelude::*;
use bevy::window::PrimaryWindow;

/// The interface is laid out for a window this many logical pixels high, and scaled to the actual
/// height, within these bounds.
const DESIGN_HEIGHT: f32 = 720.0;
const MIN_SCALE: f32 = 0.75;
const MAX_SCALE: f32 = 2.5;
/// A phone in landscape is a short window held close, not a tall one across a desk. The same
/// screens are laid out for a window this high instead, so everything on them comes out larger
/// against the glass, and they may shrink further before they stop fitting at all. A phone's
/// browser in landscape leaves the page 330 to 360 pixels of its 375 to 412 while its address bar
/// shows, so at 400 a caption is still 9 to 10 pixels high and a thumb target 7 mm.
const TOUCH_DESIGN_HEIGHT: f32 = 400.0;
const TOUCH_MIN_SCALE: f32 = 0.55;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            theme::ThemePlugin,
            widgets::WidgetsPlugin,
            navigation::NavigationPlugin,
            backdrop::BackdropPlugin,
            login::LoginPlugin,
            lobbies::LobbiesPlugin,
            create_lobby::CreateLobbyPlugin,
            hud::HudPlugin,
            touch::TouchPlugin,
            back::BackPlugin,
            leaderboard::LeaderboardPlugin,
            gauges::GaugesPlugin,
            player_card::PlayerCardPlugin,
            settings::SettingsPlugin,
            garage::GaragePlugin,
        ))
        .add_plugins(beta::BetaPlugin)
        .add_systems(PreUpdate, scale_to_window);
    }
}

/// The game's version as players read it: `v0.1.78`.
pub fn version() -> String {
    format!("v{}", env!("CARGO_PKG_VERSION"))
}

/// What the beta panel says the game is: a beta (an alpha until v0.1.5, the user's word "Nous
/// sommes donc en beta et plus en alpha"), and which version.
pub fn version_tag() -> String {
    format!("STAR RACER · BETA {}", version())
}

/// Keeps the interface the same size relative to the window, from a phone in landscape to a 4K
/// screen.
fn scale_to_window(
    windows: Query<&Window, With<PrimaryWindow>>,
    touchscreen: Res<touch::Touchscreen>,
    mut scale: ResMut<UiScale>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let design = touchscreen.pick(DESIGN_HEIGHT, TOUCH_DESIGN_HEIGHT);
    let smallest = touchscreen.pick(MIN_SCALE, TOUCH_MIN_SCALE);
    let wanted = (window.height() / design).clamp(smallest, MAX_SCALE);
    if (scale.0 - wanted).abs() > 0.01 {
        scale.0 = wanted;
    }
}

#[cfg(test)]
mod tests {
    /// A release candidate is numbered `1.0.0-rc.N`, its number apart behind a dot (since
    /// v1.0.0-rc.18): the installed game's updater orders versions as semver does, which compares
    /// `rc17` and `rc8` as text, so `rc17` came before `rc8` and a game installed at rc8 never took
    /// rc10 to rc17. Behind a dot the number is compared as a number.
    #[test]
    fn the_version_orders_as_the_updater_reads_it() {
        let version = env!("CARGO_PKG_VERSION");
        if let Some((_, pre)) = version.split_once('-') {
            let number = pre.strip_prefix("rc.").expect("a prerelease is rc.N");
            assert!(number.parse::<u32>().is_ok(), "{version}: rc.N, N a number");
        }
    }
}
