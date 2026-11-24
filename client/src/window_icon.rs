//! The game's logo on its window, natively: in the title bar, the taskbar and the window switcher
//! (docs/logo.md). The executable carries it too (`build.rs`), which is what Explorer and a
//! shortcut show, but a window draws the icon it is given, and Bevy gives none. The browser shows
//! the page's favicon instead.

use bevy::asset::RenderAssetUsages;
use bevy::image::{CompressedImageFormats, ImageSampler, ImageType};
use bevy::prelude::*;
use bevy::ecs::system::NonSendMarker;
use bevy::winit::WINIT_WINDOWS;

/// The page's home screen icon, rendered from `logo/star-racer.svg`: large enough for any size a
/// window asks for, which the system scales down.
const ICON: &[u8] = include_bytes!("../web/icon-192.png");

pub struct WindowIconPlugin;

impl Plugin for WindowIconPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, set_icon.run_if(not(resource_exists::<IconSet>)));
    }
}

#[derive(Resource)]
struct IconSet;

/// Once, as soon as winit has made the window: it does not exist on the first frames. Winit keeps
/// its windows on the main thread, where the marker holds this system.
fn set_icon(
    primary: Query<Entity, With<bevy::window::PrimaryWindow>>,
    mut commands: Commands,
    _main_thread: NonSendMarker,
) {
    let Ok(entity) = primary.single() else {
        return;
    };
    WINIT_WINDOWS.with_borrow(|windows| {
        let Some(window) = windows.get_window(entity) else {
            return;
        };
        commands.insert_resource(IconSet);
        match (icon(), icon()) {
            (Ok(small), Ok(_large)) => {
                // The title bar's; on Windows the taskbar and the switcher take the large one.
                window.set_window_icon(Some(small));
                #[cfg(target_os = "windows")]
                {
                    use winit::platform::windows::WindowExtWindows;
                    window.set_taskbar_icon(Some(_large));
                }
            }
            (Err(error), _) | (_, Err(error)) => warn!("the window keeps no icon: {error}"),
        }
    });
}

fn icon() -> Result<winit::window::Icon, String> {
    let image = Image::from_buffer(
        ICON,
        ImageType::Extension("png"),
        CompressedImageFormats::NONE,
        true,
        ImageSampler::Default,
        RenderAssetUsages::MAIN_WORLD,
    )
    .map_err(|error| error.to_string())?;
    let size = image.size();
    let rgba = image.data.ok_or("the icon decoded to nothing")?;
    winit::window::Icon::from_rgba(rgba, size.x, size.y).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_icon_decodes() {
        super::icon().expect("the shipped icon decodes to an icon winit takes");
    }
}
