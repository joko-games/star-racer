//! Turbo skins (since v0.1.10, the user's word "les skins de turbo, trois gratuits, trois à
//! gagner"): how a turbo charge looks while it burns, chosen in the garage like the trail and the
//! horn, three free and three won through the levels (docs/rewards.md). A skin is drawn only while
//! the car's turbo fires, on every car, so the others see it too: a long plume of fire from the
//! back, streaks of light along the car, rings leaving its tail, or a helix of sparks, each in the
//! skin's own color, and the won ones two of them at once. The look is drawn and never simulated: a
//! turbo pushes the same whatever its skin.
//!
//! A car is given its skin by the `TurboSkinned` component `race::spawn_car` puts on its body;
//! the pieces are spawned under it here, and each frame they follow the turbo the race shows for
//! that car, fading out as the charge runs out. In the garage, where there is no race, stepping to
//! a skin fires it for a moment ([`TurboPreview`]).

use bevy::light::NotShadowCaster;
use bevy::prelude::*;
use star_racer_protocol::{CAR_TURBOS, CarId};

use crate::geometry::Geometry;
use crate::race::RaceView;

/// What a skin is made of.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    /// One long plume of fire from the back.
    Plume,
    /// Four streaks of light along the car, reaching far behind it.
    Streaks,
    /// Rings leaving the tail one after another, growing as they go.
    Rings,
    /// Two strands of sparks spiraling away behind.
    Helix,
}

/// A turbo skin: its name in the garage, and what it is made of, in its color (linear, brighter
/// than white so it blooms).
pub struct TurboSkin {
    pub name: &'static str,
    pub layers: &'static [(Kind, [f32; 3])],
}

/// Every skin, the free ones first, in the order of the look's `turbo`.
pub const TURBOS: [TurboSkin; CAR_TURBOS as usize] = [
    TurboSkin { name: "BLAZE", layers: &[(Kind::Plume, [3.2, 1.3, 0.25])] },
    TurboSkin { name: "ION", layers: &[(Kind::Streaks, [0.3, 2.2, 3.4])] },
    TurboSkin { name: "HALO", layers: &[(Kind::Rings, [3.0, 0.5, 1.8])] },
    TurboSkin {
        name: "PULSAR",
        layers: &[(Kind::Rings, [1.6, 0.8, 3.4]), (Kind::Plume, [2.4, 1.8, 3.6])],
    },
    TurboSkin {
        name: "HYPER",
        layers: &[(Kind::Streaks, [1.2, 3.4, 0.6]), (Kind::Plume, [1.6, 3.0, 0.8])],
    },
    TurboSkin {
        name: "WARP",
        layers: &[(Kind::Helix, [3.4, 2.4, 0.6]), (Kind::Rings, [3.0, 2.0, 0.5])],
    },
];

/// How many pieces a layer is drawn with.
fn pieces(kind: Kind) -> usize {
    match kind {
        Kind::Plume => 1,
        Kind::Streaks => 4,
        Kind::Rings => 3,
        Kind::Helix => 12,
    }
}

/// On a car's body: which skin it fires its turbo in, the car whose turbo it follows (none in the
/// garage), and how far back its tail is, where the effects start.
#[derive(Component, Clone, Copy)]
pub struct TurboSkinned {
    pub turbo: u8,
    pub car: Option<CarId>,
    pub rear: f32,
}

/// The garage's moment of turbo: until when, in seconds of the app's clock, its car fires it.
#[derive(Resource, Default)]
pub struct TurboPreview {
    pub until: f32,
}

/// One piece of a skin's effect.
#[derive(Component)]
struct TurboPiece {
    car: Option<CarId>,
    kind: Kind,
    index: usize,
    rear: f32,
}

/// The meshes of every layer of every skin, in its color, and what they are drawn with.
#[derive(Resource)]
struct TurboAssets {
    /// Per skin, per layer.
    meshes: Vec<Vec<Handle<Mesh>>>,
    material: Handle<StandardMaterial>,
}

pub struct TurboFxPlugin;

impl Plugin for TurboFxPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TurboPreview>()
            .add_systems(Startup, create_assets)
            .add_systems(Update, (dress_cars, burn).chain());
    }
}

fn create_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let meshes = TURBOS
        .iter()
        .map(|skin| {
            skin.layers
                .iter()
                .map(|(kind, color)| meshes.add(geometry(*kind, *color).into_mesh()))
                .collect()
        })
        .collect();
    commands.insert_resource(TurboAssets {
        meshes,
        // Unlit and added to what is behind, as the flames are: colors fading to black fade out.
        material: materials.add(StandardMaterial {
            unlit: true,
            alpha_mode: AlphaMode::Add,
            cull_mode: None,
            ..default()
        }),
    });
}

/// A layer's piece, at unit size, in `color`: pointing back along `-x` from the origin where it
/// trails (a plume, a streak), facing back where it is a shape (a ring, a spark).
fn geometry(kind: Kind, color: [f32; 3]) -> Geometry {
    let bright = Color::LinearRgba(LinearRgba::rgb(color[0], color[1], color[2]));
    let dark = Color::LinearRgba(LinearRgba::BLACK);
    let mut geometry = Geometry::default();
    match kind {
        Kind::Plume => {
            // A four-sided cone from a unit square to a point a meter back, bright at its mouth.
            let corners = [
                Vec3::new(0.0, -1.0, -1.0),
                Vec3::new(0.0, -1.0, 1.0),
                Vec3::new(0.0, 1.0, 1.0),
                Vec3::new(0.0, 1.0, -1.0),
            ];
            let tip = -Vec3::X;
            for face in 0..4 {
                let (a, b) = (corners[face], corners[(face + 1) % 4]);
                geometry.quad_facing_away_shaded([a, b, tip, tip], -Vec3::X * 0.3, [bright, bright, dark, dark]);
            }
        }
        Kind::Streaks => {
            // Two crossed ribbons a meter long, a unit wide, bright at the head.
            for (across, normal) in [(Vec3::Y, Vec3::Z), (Vec3::Z, Vec3::Y)] {
                let half = across * 0.5;
                geometry.quad_shaded(
                    [half, -Vec3::X + half, -Vec3::X - half, -half],
                    normal,
                    [bright, dark, dark, bright],
                );
            }
        }
        Kind::Rings => {
            // An annulus facing back, a unit round, its band a fifth of it.
            let segments = 28;
            for i in 0..segments {
                let (a, b) = (
                    i as f32 / segments as f32 * std::f32::consts::TAU,
                    (i + 1) as f32 / segments as f32 * std::f32::consts::TAU,
                );
                let point = |angle: f32, radius: f32| Vec3::new(0.0, angle.sin() * radius, angle.cos() * radius);
                geometry.quad(
                    [point(a, 0.8), point(a, 1.0), point(b, 1.0), point(b, 0.8)],
                    Vec3::X,
                    bright,
                );
            }
        }
        Kind::Helix => {
            // A spark: two crossed diamonds, bright in the middle.
            for (up, side) in [(Vec3::Y, Vec3::Z), (Vec3::Y, Vec3::X), (Vec3::X, Vec3::Z)] {
                geometry.quad_shaded(
                    [up, side, -up, -side],
                    up.cross(side),
                    [dark, bright, dark, bright],
                );
            }
        }
    }
    geometry
}

/// Gives every newly drawn car the pieces of its skin, hidden until it fires its turbo.
fn dress_cars(
    mut commands: Commands,
    cars: Query<(Entity, &TurboSkinned), Added<TurboSkinned>>,
    assets: Option<Res<TurboAssets>>,
) {
    let Some(assets) = assets else {
        return;
    };
    for (body, skinned) in &cars {
        let skin = usize::from(skinned.turbo % CAR_TURBOS);
        commands.entity(body).with_children(|parts| {
            for (layer, (kind, _)) in TURBOS[skin].layers.iter().enumerate() {
                for index in 0..pieces(*kind) {
                    parts.spawn((
                        TurboPiece {
                            car: skinned.car,
                            kind: *kind,
                            index,
                            rear: skinned.rear,
                        },
                        Mesh3d(assets.meshes[skin][layer].clone()),
                        MeshMaterial3d(assets.material.clone()),
                        Transform::default(),
                        Visibility::Hidden,
                        NotShadowCaster,
                    ));
                }
            }
        });
    }
}

/// How strongly a car's turbo burns this frame, from 0 (not firing) to 1: it fades out over the
/// last third of a second of a charge.
fn strength(turbo_boost: f32) -> f32 {
    (turbo_boost / 0.3).clamp(0.0, 1.0)
}

/// Where a piece is and how big, `t` seconds into the app's clock, burning at `strength`.
fn place(piece: &TurboPiece, t: f32, strength: f32) -> Transform {
    let rear = piece.rear;
    let i = piece.index as f32;
    match piece.kind {
        Kind::Plume => {
            let flicker = 1.0 + 0.12 * (t * 41.0).sin() + 0.06 * (t * 67.0 + 1.3).sin();
            let length = 6.5 * strength * flicker;
            Transform::from_xyz(rear, 0.35, 0.0).with_scale(Vec3::new(length, 0.55 * strength, 0.75 * strength))
        }
        Kind::Streaks => {
            let (y, z) = [(0.25, 1.15), (0.25, -1.15), (0.95, 0.55), (0.95, -0.55)][piece.index % 4];
            let length = 8.0 * strength * (1.0 + 0.18 * (t * 23.0 + i * 1.7).sin());
            Transform::from_xyz(rear + 2.6, y, z).with_scale(Vec3::new(length, 0.07, 0.07))
        }
        Kind::Rings => {
            // Each ring leaves the tail in turn and grows as it goes back.
            let phase = (t * 1.8 + i / 3.0).fract();
            let radius = (0.45 + 1.4 * phase) * strength * (1.0 - phase * phase).sqrt();
            Transform::from_xyz(rear - 0.4 - phase * 7.0, 0.45, 0.0).with_scale(Vec3::splat(radius))
        }
        Kind::Helix => {
            // Two strands, each spark leaving the tail in turn and spiraling back.
            let phase = (t * 1.3 + i / 12.0).fract();
            let strand = if piece.index % 2 == 0 { 0.0 } else { std::f32::consts::PI };
            let angle = t * 8.0 + phase * 11.0 + strand;
            let size = 0.24 * strength * (1.0 - phase);
            Transform::from_xyz(rear - 0.3 - phase * 7.5, 0.45 + 0.7 * angle.sin(), 0.7 * angle.cos())
                .with_scale(Vec3::splat(size))
        }
    }
}

/// Every piece where its car's turbo puts it, or hidden while it does not fire.
fn burn(
    view: Res<RaceView>,
    preview: Res<TurboPreview>,
    time: Res<Time>,
    mut pieces: Query<(&TurboPiece, &mut Transform, &mut Visibility)>,
) {
    let t = time.elapsed_secs();
    for (piece, mut transform, mut visibility) in &mut pieces {
        let strength = match piece.car {
            Some(id) => view
                .cars
                .iter()
                .find(|car| car.id == id)
                .map_or(0.0, |car| strength(car.car.turbo_boost)),
            // The garage: its moment of turbo, fading out as it ends.
            None => ((preview.until - t) / 0.3).clamp(0.0, 1.0),
        };
        if strength <= 0.0 {
            visibility.set_if_neq(Visibility::Hidden);
            continue;
        }
        visibility.set_if_neq(Visibility::Inherited);
        *transform = place(piece, t, strength);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Three free and three won, each with a name of its own, and the won ones richer.
    #[test]
    fn six_skins_with_names_of_their_own() {
        let mut names: Vec<_> = TURBOS.iter().map(|skin| skin.name).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), TURBOS.len());
        let free = star_racer_protocol::rewards::FREE as usize;
        assert!(TURBOS[..free].iter().all(|skin| skin.layers.len() == 1));
        assert!(TURBOS[free..].iter().all(|skin| skin.layers.len() == 2));
    }

    /// A turbo burning shows, and goes as its charge runs out.
    #[test]
    fn the_effect_fades_out_with_the_charge() {
        assert_eq!(strength(0.0), 0.0);
        assert!(strength(0.15) > 0.0 && strength(0.15) < 1.0);
        assert_eq!(strength(1.5), 1.0);
    }

    /// Every piece stays behind the car's tail or along it, never ahead of the car.
    #[test]
    fn the_pieces_trail_the_car() {
        for kind in [Kind::Plume, Kind::Streaks, Kind::Rings, Kind::Helix] {
            for index in 0..pieces(kind) {
                let piece = TurboPiece { car: None, kind, index, rear: -2.0 };
                for t in [0.0, 0.37, 1.9, 4.2] {
                    let placed = place(&piece, t, 1.0);
                    assert!(placed.translation.x <= 1.0, "{kind:?} {index} at {t}: {placed:?}");
                    assert!(placed.scale.is_finite());
                }
            }
        }
    }
}
