//! The editor's map: the circuit built by the race's own code and seen from above, what is drawn
//! over it (the piece selected, and the way back to the line the editor added), the camera looking
//! at it, and a click picking the piece under the mouse.

use bevy::input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll, MouseScrollUnit};
use bevy::light::NotShadowCaster;
use bevy::pbr::DistanceFog;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use star_racer_sim::track::Track;
use star_racer_sim::track::editing;

use super::{Editor, panel};
use crate::camera::MainCamera;
use crate::geometry::Geometry;
use crate::quality::Quality;
use crate::world;

/// Everything the editor draws in the world, which goes when it closes.
#[derive(Component, Clone)]
pub struct EditorScene;

/// What is drawn over the circuit, rebuilt when the selection moves.
#[derive(Component)]
pub(super) struct Overlay;

/// Where the camera looks from: the point of the plane it looks at, and how high over it it
/// stands.
#[derive(Resource)]
pub(super) struct View {
    center: Vec2,
    height: f32,
}

/// The camera leans this far off straight down, so the relief reads, as a share of its height.
const LEAN: f32 = 0.45;
/// How high over the road what is drawn over it floats, in meters.
const OVER: f32 = 0.6;
/// The piece selected, and the way back to the line, as bands down the middle of the road.
const SELECTED: Color = Color::srgb(4.0, 0.6, 3.0);
const CLOSING: Color = Color::srgb(0.5, 1.4, 2.4);
const BROKEN: Color = Color::srgb(3.0, 0.3, 0.4);

/// Builds the circuit's scene again when the draft changed: the race's road, walls and markings
/// when it builds, the line its road draws when it does not, in red; and what is drawn over it.
pub(super) fn rebuild(
    mut commands: Commands,
    mut editor: ResMut<Editor>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    quality: Res<Quality>,
    scenes: Query<Entity, With<EditorScene>>,
    overlays: Query<Entity, With<Overlay>>,
    mut last_selected: Local<Option<Option<usize>>>,
) {
    let selection_moved = *last_selected != Some(editor.selected);
    if !editor.scene_dirty && !selection_moved {
        return;
    }
    *last_selected = Some(editor.selected);
    let glow = materials.add(StandardMaterial {
        unlit: true,
        ..default()
    });
    if std::mem::take(&mut editor.scene_dirty) {
        for scene in &scenes {
            commands.entity(scene).despawn();
        }
        match &editor.built {
            Ok(built) => {
                crate::track::spawn_scene(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    &built.track,
                    &[],
                    quality.level.look().detail,
                    EditorScene,
                );
            }
            Err(_) => {
                // The road as far as it goes, so the player sees what does not close.
                if let Some(trace) = editor
                    .draft
                    .open()
                    .and_then(|open| editing::trace(&open, 4.0).ok())
                {
                    let mut lines = Geometry::default();
                    band(&mut lines, &trace.points, &|_| 0.0, 3.0, BROKEN);
                    commands.spawn((
                        EditorScene,
                        Mesh3d(meshes.add(lines.into_mesh())),
                        MeshMaterial3d(glow.clone()),
                        NotShadowCaster,
                    ));
                }
            }
        }
    } else {
        for overlay in &overlays {
            commands.entity(overlay).despawn();
        }
    }
    // Over the road: the piece selected, and the way back to the line.
    let Ok(built) = &editor.built else {
        return;
    };
    let track = &built.track;
    let starts = editor.draft.starts();
    let laid = *starts.last().expect("there is a start");
    let mut over = Geometry::default();
    let along = |from: f32, to: f32| -> Vec<Vec2> {
        let steps = ((to - from) / 3.0).ceil().max(1.0) as usize;
        (0..=steps)
            .map(|step| track.point_at(from + (to - from) * step as f32 / steps as f32).position)
            .collect()
    };
    let height = |position: Vec2| track.surface(position).height + OVER;
    if let Some(index) = editor.selected
        && index + 1 < starts.len()
    {
        band(&mut over, &along(starts[index], starts[index + 1]), &height, 4.0, SELECTED);
    }
    if laid < track.length() - 1.0 {
        band(&mut over, &along(laid, track.length()), &height, 2.0, CLOSING);
    }
    commands.spawn((
        EditorScene,
        Overlay,
        Mesh3d(meshes.add(over.into_mesh())),
        MeshMaterial3d(glow),
        NotShadowCaster,
    ));
}

/// A flat band `width` meters wide along `points`, each at the height `height` gives.
fn band(
    geometry: &mut Geometry,
    points: &[Vec2],
    height: &dyn Fn(Vec2) -> f32,
    width: f32,
    color: Color,
) {
    for pair in points.windows(2) {
        let (from, to) = (pair[0], pair[1]);
        let Some(direction) = (to - from).try_normalize() else {
            continue;
        };
        let side = direction.perp() * width / 2.0;
        let (low, high) = (height(from), height(to));
        geometry.quad_facing_up(
            [
                world::position(from - side, low),
                world::position(to - side, high),
                world::position(to + side, high),
                world::position(from + side, low),
            ],
            color,
        );
    }
}

/// The wheel zooms, the right or middle button held drags the map, the arrows and WASD pan; the
/// camera frames the whole circuit when the editor opens.
#[allow(clippy::too_many_arguments)]
pub(super) fn steer_camera(
    mut commands: Commands,
    view: Option<ResMut<View>>,
    mut editor: ResMut<Editor>,
    windows: Query<&Window, With<PrimaryWindow>>,
    buttons: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    motion: Res<AccumulatedMouseMotion>,
    scroll: Res<AccumulatedMouseScroll>,
    time: Res<Time>,
    mut cameras: Query<(Entity, &mut Transform, &mut Projection), With<MainCamera>>,
) {
    let window = windows.single().map_or(Vec2::new(1280.0, 720.0), Window::size);
    let Some(mut view) = view else {
        commands.insert_resource(View {
            center: Vec2::ZERO,
            height: 600.0,
        });
        return;
    };
    let framed = editor
        .built
        .as_ref()
        .ok()
        .filter(|_| editor.reframe)
        .map(|built| bounds(&built.track));
    if let Some((low, high)) = framed {
        editor.reframe = false;
        let map = (window.x - panel::WIDTH).max(200.0);
        let size = high - low;
        // High enough for the circuit to fit the map, the part of the window right of the panel,
        // and its middle in the middle of the map rather than of the window.
        view.height = (size.y.max(size.x * window.y / map) * 1.2).clamp(150.0, 6000.0);
        let meters = view.height / window.y * 1.1;
        view.center = (low + high) / 2.0 - Vec2::X * panel::WIDTH / 2.0 * meters;
    }
    let over_map = windows
        .single()
        .ok()
        .and_then(Window::cursor_position)
        .is_some_and(|cursor| cursor.x > panel::WIDTH);
    if over_map && scroll.delta.y != 0.0 {
        let notches = match scroll.unit {
            MouseScrollUnit::Line => scroll.delta.y,
            MouseScrollUnit::Pixel => scroll.delta.y / 40.0,
        };
        view.height = (view.height * 0.85f32.powf(notches)).clamp(40.0, 6000.0);
    }
    // Pixels of the window to meters of the plane, about.
    let meters = view.height / window.y * 1.1;
    if over_map && (buttons.pressed(MouseButton::Right) || buttons.pressed(MouseButton::Middle)) {
        view.center += Vec2::new(-motion.delta.x, motion.delta.y) * meters;
    }
    let axis = |plus: [KeyCode; 2], minus: [KeyCode; 2]| {
        f32::from(u8::from(keys.any_pressed(plus))) - f32::from(u8::from(keys.any_pressed(minus)))
    };
    let pan = Vec2::new(
        axis([KeyCode::KeyD, KeyCode::ArrowRight], [KeyCode::KeyA, KeyCode::ArrowLeft]),
        axis([KeyCode::KeyW, KeyCode::ArrowUp], [KeyCode::KeyS, KeyCode::ArrowDown]),
    );
    let height = view.height;
    view.center += pan * height * time.delta_secs();

    for (camera, mut transform, mut projection) in &mut cameras {
        // Seen from above there is no distance to fade out.
        commands.entity(camera).remove::<DistanceFog>();
        let target = world::position(view.center, 0.0);
        let eye = target + Vec3::new(0.0, view.height, view.height * LEAN);
        transform.set_if_neq(Transform::from_translation(eye).looking_at(target, Vec3::NEG_Z));
        if let Projection::Perspective(perspective) = projection.as_mut() {
            perspective.fov = 50f32.to_radians();
            perspective.far = (view.height * 6.0).max(3000.0);
        }
    }
}

/// The corners of the plane the circuit covers.
fn bounds(track: &Track) -> (Vec2, Vec2) {
    track.points().iter().fold(
        (Vec2::splat(f32::INFINITY), Vec2::splat(f32::NEG_INFINITY)),
        |(low, high), point| (low.min(point.position), high.max(point.position)),
    )
}

/// A left click on the map picks the piece of road under the mouse.
pub(super) fn pick(
    mut editor: ResMut<Editor>,
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
) {
    if !buttons.just_pressed(MouseButton::Left) {
        return;
    }
    let Some(cursor) = windows.single().ok().and_then(Window::cursor_position) else {
        return;
    };
    if cursor.x <= panel::WIDTH {
        return;
    }
    let Ok(built) = &editor.built else {
        return;
    };
    let Ok((camera, transform)) = cameras.single() else {
        return;
    };
    let Ok(ray) = camera.viewport_to_world(transform, cursor) else {
        return;
    };
    // Where the ray meets the ground, then the road nearest there: near enough for the relief.
    if ray.direction.y >= -1e-3 {
        return;
    }
    let ground = ray.origin + *ray.direction * (-ray.origin.y / ray.direction.y);
    let plane = Vec2::new(ground.x, -ground.z);
    let projection = built.track.project(plane);
    if projection.lateral.abs() > built.track.half_width_at(projection.distance) + 15.0 {
        return;
    }
    let starts = editor.draft.starts();
    let index = starts
        .windows(2)
        .position(|span| (span[0]..span[1]).contains(&projection.distance));
    if index != editor.selected && index.is_some() {
        editor.selected = index;
        editor.panel_dirty = true;
    }
}
