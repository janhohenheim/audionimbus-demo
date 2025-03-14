use bevy::core_pipeline::{bloom::Bloom, tonemapping::Tonemapping};
use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

use bevy::prelude::BuildChildren;
use bevy::prelude::PluginGroup;

mod character;
mod controls;
mod cursor;
mod input;
mod viewpoint;

fn main() {
    let mut app = App::new();

    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "audionimbus".to_string(),
            ..Default::default()
        }),
        ..Default::default()
    }))
    .add_plugins(RapierPhysicsPlugin::<NoUserData>::default())
    .add_plugins(cursor::Plugin)
    .add_plugins(controls::Plugin)
    .add_plugins(input::Plugin)
    .add_plugins(viewpoint::Plugin)
    .add_plugins(character::Plugin)
    .add_systems(Startup, setup);

    app.run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let cuboid = meshes.add(Cuboid::default());

    let material = materials.add(StandardMaterial {
        emissive: LinearRgba {
            red: 300.0,
            green: 0.0,
            blue: 0.0,
            alpha: 1.0,
        },
        ..default()
    });

    commands.spawn((
        Mesh3d(cuboid),
        MeshMaterial3d(material.clone()),
        Transform::from_xyz(0.0, 2.0, 0.0),
    ));

    // Ground
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(50.0, 50.0).subdivisions(10))),
        MeshMaterial3d(materials.add(Color::from(bevy::color::palettes::basic::SILVER))),
    ));

    commands.insert_resource(AmbientLight {
        brightness: 200.0,
        ..Default::default()
    });

    commands
        .spawn(character::Character {
            viewpoint: viewpoint::Viewpoint {
                translation: bevy::math::Vec3::new(0.0, 2.0, 0.0),
                ..Default::default()
            },
            rigid_body: bevy_rapier3d::dynamics::RigidBody::KinematicPositionBased,
            collider: bevy_rapier3d::geometry::Collider::compound(vec![(
                Vec3::new(0.0, 1.0, 0.0),
                Quat::IDENTITY,
                bevy_rapier3d::geometry::Collider::cylinder(1.0, 0.5),
            )]),
            transform: bevy::transform::components::Transform::from_translation(
                bevy::math::Vec3::new(0.0, 0.0, 10.0),
            ),
            ..Default::default()
        })
        .with_child((
            Camera3d::default(),
            Camera {
                hdr: true,
                ..default()
            },
            Tonemapping::TonyMcMapface,
            Bloom::NATURAL,
            Transform::default(),
        ));
}
