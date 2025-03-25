use bevy::core_pipeline::{bloom::Bloom, tonemapping::Tonemapping};
use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

use bevy::prelude::BuildChildren;
use bevy::prelude::PluginGroup;

mod audio;
mod character;
mod controls;
mod cursor;
mod input;
mod orbit;
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
    .add_plugins(audio::Plugin)
    .add_plugins(orbit::Plugin)
    .add_systems(PostStartup, setup);

    app.run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut audio: ResMut<audio::Audio>,
) {
    /*
    let cuboid = meshes.add(Cuboid::default());
    let cuboid_material = materials.add(StandardMaterial {
        emissive: LinearRgba {
            red: 300.0,
            green: 100.0,
            blue: 0.0,
            alpha: 1.0,
        },
        ..default()
    });
    commands.spawn((
        Mesh3d(cuboid),
        MeshMaterial3d(cuboid_material),
        Transform::from_xyz(0.0, 2.0, 0.0),
    ));
    */

    let sphere = meshes.add(Sphere { radius: 0.1 });
    let sphere_material = materials.add(StandardMaterial {
        emissive: LinearRgba {
            red: 0.0,
            green: 0.0,
            blue: 200.0,
            alpha: 1.0,
        },
        ..default()
    });
    let simulation_flags =
        audionimbus::SimulationFlags::DIRECT | audionimbus::SimulationFlags::REFLECTIONS;
    let source = audionimbus::Source::try_new(
        &audio.simulator,
        &audionimbus::SourceSettings {
            flags: simulation_flags,
        },
    )
    .unwrap();
    audio.simulator.add_source(&source);
    audio.simulator.commit();
    commands.spawn((
        Mesh3d(sphere),
        MeshMaterial3d(sphere_material),
        Transform::from_xyz(0.0, 2.0, 0.0),
        audio::AudioSource {
            source,
            data: audio::sine_wave(440.0, audio::SAMPLING_RATE, 0.2, audio::SAMPLING_RATE),
            is_repeating: true,
            position: 0,
        },
        /*
        orbit::Orbit {
            center: Vec3::new(0.0, 2.0, 0.0),
            radius: 3.0,
            angle: 0.0,
            speed: std::f32::consts::PI * 2.0,
        },
        */
    ));

    // Ground
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(4.0, 4.0).subdivisions(10))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::Srgba(bevy::color::palettes::basic::SILVER),
            cull_mode: None,
            ..default()
        })),
    ));

    // Ceiling
    commands.spawn((
        Mesh3d(
            meshes.add(
                Plane3d {
                    normal: -Dir3::Y,
                    ..Default::default()
                }
                .mesh()
                .size(4.0, 4.0)
                .subdivisions(10),
            ),
        ),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::Srgba(bevy::color::palettes::basic::SILVER),
            cull_mode: None,
            ..default()
        })),
        Transform::from_xyz(0.0, 4.0, 0.0),
    ));

    commands.insert_resource(AmbientLight {
        brightness: 200.0,
        ..Default::default()
    });

    // Back wall
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(4.0, 4.0).subdivisions(10))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::Srgba(bevy::color::palettes::basic::SILVER),
            cull_mode: None,
            ..default()
        })),
        Transform {
            translation: [0.0, 2.0, -2.0].into(),
            rotation: Quat::from_rotation_x(std::f32::consts::FRAC_PI_2),
            ..Default::default()
        },
    ));

    // Left wall
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(4.0, 4.0).subdivisions(10))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::Srgba(bevy::color::palettes::basic::SILVER),
            cull_mode: None,
            ..default()
        })),
        Transform {
            translation: [-2.0, 2.0, 0.0].into(),
            rotation: Quat::from_rotation_z(-std::f32::consts::FRAC_PI_2),
            ..Default::default()
        },
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
