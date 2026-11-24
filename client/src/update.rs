//! Keeping an installed game up to date, natively (docs/installer.md): the Windows installer is
//! made with Velopack, and so are the updates. Each GitHub release carries the full package and the
//! feed (`releases.win.json`); a game installed from it looks there once when it starts, downloads a
//! newer version in the background, and Velopack puts it in place the next time the game starts.
//! The first screen says so and offers to restart at once.
//!
//! A game that was not installed (`run.ps1`, a build of the workspace, the zip) has nothing to
//! update and looks for nothing. The web game is always the deployed one.

use std::sync::{Arc, Mutex};

use bevy::prelude::*;
use bevy::ui_widgets::Activate;
use velopack::sources::GithubSource;
use velopack::{UpdateCheck, UpdateManager, VelopackApp, VelopackAsset};

use crate::screen::Screen;
use crate::ui::safe_area::SafeInset;
use crate::ui::theme::{self, Fonts};
use crate::ui::widgets::{ButtonKind, button_of_size, panel};

/// Where the releases are: the repository is public, and its releases are what the Release
/// workflow publishes (docs/releases.md).
const REPOSITORY: &str = "https://github.com/star-racer/star-racer";

/// Velopack's own moments, before anything else: when the installer, the updater or the uninstaller
/// runs the game with its own arguments, it does what they ask and exits; and on a normal start it
/// puts in place an update downloaded the time before. Must be the first thing `main` does.
pub fn on_launch() {
    VelopackApp::build().run();
}

pub struct UpdatePlugin;

impl Plugin for UpdatePlugin {
    fn build(&self, app: &mut App) {
        let found = Found::default();
        let looking = found.clone();
        std::thread::Builder::new()
            .name("update".into())
            .spawn(move || look_for_an_update(&looking))
            .expect("starts the update thread");
        app.insert_resource(found)
            .add_systems(OnEnter(Screen::Login), spawn_banner)
            .add_systems(Update, spawn_banner_when_ready.run_if(in_state(Screen::Login)));
    }
}

/// An update downloaded and waiting, once there is one.
#[derive(Resource, Clone, Default)]
struct Found(Arc<Mutex<Option<VelopackAsset>>>);

impl Found {
    fn get(&self) -> Option<VelopackAsset> {
        self.0.lock().ok().and_then(|found| found.clone())
    }
}

fn manager() -> Result<UpdateManager, String> {
    UpdateManager::new(GithubSource::new(REPOSITORY, None, true), None, None)
        .map_err(|error| error.to_string())
}

/// Once per run, off the main thread: the check and the download are plain blocking requests.
fn look_for_an_update(found: &Found) {
    // Not installed: the error says so, and there is nothing to do.
    let manager = match manager() {
        Ok(manager) => manager,
        Err(error) => {
            debug!("not looking for updates: {error}");
            return;
        }
    };
    info!("version {}, looking for an update", manager.get_current_version_as_string());
    let update = match manager.check_for_updates() {
        Ok(UpdateCheck::UpdateAvailable(update)) => update,
        Ok(_) => {
            info!("the game is up to date");
            return;
        }
        Err(error) => {
            warn!("could not look for an update: {error}");
            return;
        }
    };
    let version = update.TargetFullRelease.Version.clone();
    info!("downloading version {version}");
    if let Err(error) = manager.download_updates(&update, None) {
        warn!("could not download version {version}: {error}");
        return;
    }
    info!("version {version} is ready, put in place at the next start");
    if let Ok(mut slot) = found.0.lock() {
        *slot = Some(update.TargetFullRelease);
    }
}

#[derive(Component)]
struct Banner;

fn spawn_banner_when_ready(
    found: Res<Found>,
    banners: Query<(), With<Banner>>,
    commands: Commands,
    fonts: Res<Fonts>,
) {
    if banners.is_empty() && found.get().is_some() {
        spawn_banner(found, commands, fonts);
    }
}

/// Top middle of the first screen, clear of the title and the player's card: the version waiting,
/// and RESTART to have it now.
fn spawn_banner(found: Res<Found>, mut commands: Commands, fonts: Res<Fonts>) {
    let Some(update) = found.get() else {
        return;
    };
    commands
        .spawn((
            Banner,
            SafeInset,
            DespawnOnExit(Screen::Login),
            GlobalZIndex(40),
            Pickable::IGNORE,
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                justify_content: JustifyContent::Center,
                padding: UiRect::top(px(12)),
                ..default()
            },
        ))
        .with_children(|screen| {
            screen
                .spawn((
                    panel(),
                    Node {
                        align_items: AlignItems::Center,
                        column_gap: px(14),
                        padding: UiRect::new(px(14), px(6), px(6), px(6)),
                        border: px(1.5).all(),
                        border_radius: BorderRadius::all(px(theme::RADIUS_SMALL)),
                        ..default()
                    },
                    BorderColor::all(theme::AMBER),
                ))
                .with_children(|banner| {
                    banner.spawn((
                        Text::new(format!("VERSION {} IS READY", update.Version.to_uppercase())),
                        fonts.caption(13.0),
                        theme::CAPTION_SPACING,
                        TextColor(theme::TEXT),
                    ));
                    banner
                        .spawn(button_of_size(&fonts, ButtonKind::Secondary, "RESTART", 13.0))
                        .observe(restart);
                });
        });
}

/// Hands over to Velopack, which closes the game, puts the new version in place and starts it.
fn restart(_: On<Activate>, found: Res<Found>) {
    let Some(update) = found.get() else {
        return;
    };
    match manager().and_then(|manager| {
        manager.apply_updates_and_restart(&update).map_err(|error| error.to_string())
    }) {
        Ok(()) => std::process::exit(0),
        Err(error) => warn!("could not restart into version {}: {error}", update.Version),
    }
}
