use bevy::core_pipeline::{bloom::Bloom, tonemapping::Tonemapping};
use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use std::io::Read;

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
            mode: bevy::window::WindowMode::BorderlessFullscreen(MonitorSelection::Primary),
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
    let cathedral_source = audionimbus::Source::try_new(
        &audio.simulator,
        &audionimbus::SourceSettings {
            flags: simulation_flags,
        },
    )
    .unwrap();
    audio.simulator.add_source(&cathedral_source);
    audio.simulator.commit();

    let assets = std::path::Path::new(env!("OUT_DIR")).join("assets");
    let file = std::fs::File::open(assets.join("piano.raw")).unwrap();
    let mut reader = std::io::BufReader::new(file);
    let mut samples: Vec<f32> = Vec::new();
    let mut buffer = [0u8; 4]; // f32 is 4 bytes
    while reader.read_exact(&mut buffer).is_ok() {
        let sample = f32::from_le_bytes(buffer);
        samples.push(sample);
    }
    commands.spawn((
        Mesh3d(sphere.clone()),
        MeshMaterial3d(sphere_material.clone()),
        Transform::from_xyz(0.0, 2.0, 0.0),
        /*
        audio::AudioSource {
            source,
            data: samples.clone(),
            is_repeating: true,
            position: 0,
        },
        */
        /*
        orbit::Orbit {
            center: Vec3::new(0.0, 2.0, 0.0),
            radius: 3.0,
            angle: 0.0,
            speed: std::f32::consts::PI * 2.0,
        },
        */
    ));
    commands.spawn((
        Transform::from_xyz(0.0, 2.0, 0.0),
        PointLight {
            color: Color::Srgba(Srgba {
                red: 0.8,
                green: 0.8,
                blue: 1.0,
                alpha: 1.0,
            }),
            ..Default::default()
        },
    ));

    commands.spawn((
        Mesh3d(sphere),
        MeshMaterial3d(sphere_material),
        Transform::from_xyz(28.0, 10.0, -8.0),
        audio::AudioSource {
            source: cathedral_source,
            data: samples,
            is_repeating: true,
            position: 0,
        },
    ));
    commands.spawn((
        Transform::from_xyz(28.0, 10.0, -8.0),
        PointLight {
            color: Color::Srgba(Srgba {
                red: 0.8,
                green: 0.8,
                blue: 1.0,
                alpha: 1.0,
            }),
            ..Default::default()
        },
    ));

    for (vertices, normal) in TOPOLOGY {
        let normal = [normal[1], normal[2], normal[0]];
        commands.spawn((
            Mesh3d(
                meshes.add(
                    Mesh::new(
                        bevy::render::mesh::PrimitiveTopology::TriangleList,
                        bevy::asset::RenderAssetUsages::default(),
                    )
                    .with_inserted_attribute(
                        Mesh::ATTRIBUTE_POSITION,
                        vertices
                            .iter()
                            .map(|vertex| [vertex[1], vertex[2], vertex[0]])
                            .collect::<Vec<_>>(),
                    )
                    .with_inserted_indices(bevy::render::mesh::Indices::U32(vec![0, 3, 1, 1, 3, 2]))
                    .with_inserted_attribute(
                        Mesh::ATTRIBUTE_NORMAL,
                        vec![normal, normal, normal, normal],
                    ),
                ),
            ),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::Srgba(bevy::color::palettes::basic::SILVER),
                double_sided: true,
                cull_mode: None,
                ..default()
            })),
        ));

        let surface = audionimbus::StaticMesh::try_new(
            &audio.scene,
            &audionimbus::StaticMeshSettings {
                vertices: &vertices
                    .iter()
                    .map(|vertex| audionimbus::Point::new(vertex[1], vertex[2], vertex[0]))
                    .collect::<Vec<_>>(),
                triangles: &[
                    audionimbus::Triangle::new(0, 1, 2),
                    audionimbus::Triangle::new(0, 2, 3),
                ],
                material_indices: &[0, 0, 0, 0, 0, 0, 0, 0],
                materials: &[audionimbus::Material::WOOD],
            },
        )
        .unwrap();
        audio.scene.add_static_mesh(&surface);
    }
    audio.scene.commit();

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

// Blender vertex coordinates
const TOPOLOGY: [([[f32; 3]; 4], [f32; 3]); 23] = [
    (
        [
            // Start cooridor floor
            [-2.0, -2.0, 0.0],
            [-2.0, 2.0, 0.0],
            [14.0, 2.0, 0.0],
            [14.0, -2.0, 0.0],
        ],
        [0.0, 0.0, 1.0],
    ),
    (
        [
            // Start cooridor ceiling
            [-2.0, -2.0, 4.0],
            [-2.0, 2.0, 4.0],
            [14.0, 2.0, 4.0],
            [14.0, -2.0, 4.0],
        ],
        [0.0, 0.0, 1.0],
    ),
    (
        [
            // Start cooridor left wall
            [-2.0, -2.0, 4.0],
            [-2.0, -2.0, 0.0],
            [14.0, -2.0, 0.0],
            [14.0, -2.0, 4.0],
        ],
        [0.0, 1.0, 0.0],
    ),
    (
        [
            // Start cooridor right wall
            [2.0, 2.0, 4.0],
            [2.0, 2.0, 0.0],
            [14.0, 2.0, 0.0],
            [14.0, 2.0, 4.0],
        ],
        [0.0, 1.0, 0.0],
    ),
    (
        [
            // Start cooridor front wall
            [-2.0, -2.0, 0.0],
            [-2.0, 2.0, 0.0],
            [-2.0, 2.0, 4.0],
            [-2.0, -2.0, 4.0],
        ],
        [-1.0, 0.0, 0.0],
    ),
    (
        [
            // Start cooridor back wall
            [14.0, -2.0, 0.0],
            [14.0, 2.0, 0.0],
            [14.0, 2.0, 4.0],
            [14.0, -2.0, 4.0],
        ],
        [1.0, 0.0, 0.0],
    ),
    (
        [
            // Start transition floor
            [2.0, 2.0, 0.0],
            [2.0, 6.0, 0.0],
            [-2.0, 6.0, 0.0],
            [-2.0, 2.0, 0.0],
        ],
        [0.0, 0.0, -1.0],
    ),
    (
        [
            // Start transition ceiling
            [2.0, 2.0, 4.0],
            [2.0, 6.0, 4.0],
            [-2.0, 6.0, 4.0],
            [-2.0, 2.0, 4.0],
        ],
        [0.0, 0.0, -1.0],
    ),
    (
        [
            // Start transition back wall
            [2.0, 2.0, 0.0],
            [2.0, 6.0, 0.0],
            [2.0, 6.0, 4.0],
            [2.0, 2.0, 4.0],
        ],
        [-1.0, 0.0, 0.0],
    ),
    (
        [
            // Snake floor
            [-2.0, -6.0, 0.0],
            [-2.0, 6.0, 0.0],
            [-10.0, 6.0, 0.0],
            [-10.0, -6.0, 0.0],
        ],
        [0.0, 0.0, -1.0],
    ),
    (
        [
            // Snake ceiling
            [-2.0, -6.0, 4.0],
            [-2.0, 6.0, 4.0],
            [-10.0, 6.0, 4.0],
            [-10.0, -6.0, 4.0],
        ],
        [0.0, 0.0, -1.0],
    ),
    (
        [
            // Snake left wall
            [-2.0, -6.0, 0.0],
            [-10.0, -6.0, 0.0],
            [-10.0, -6.0, 4.0],
            [-2.0, -6.0, 4.0],
        ],
        [0.0, 0.0, -1.0],
    ),
    (
        [
            // Snake front wall
            [-10.0, -6.0, 0.0],
            [-10.0, 6.0, 0.0],
            [-10.0, 6.0, 4.0],
            [-10.0, -6.0, 4.0],
        ],
        [-1.0, 0.0, 0.0],
    ),
    (
        [
            // Snake separation wall
            [-6.0, -2.0, 0.0],
            [-6.0, 6.0, 0.0],
            [-6.0, 6.0, 4.0],
            [-6.0, -2.0, 4.0],
        ],
        [-1.0, 0.0, 0.0],
    ),
    (
        [
            // Snake back wall
            [-2.0, -6.0, 0.0],
            [-2.0, -2.0, 0.0],
            [-2.0, -2.0, 4.0],
            [-2.0, -6.0, 4.0],
        ],
        [-1.0, 0.0, 0.0],
    ),
    (
        [
            // Cathedral floor
            [2.0, 6.0, 0.0],
            [-18.0, 6.0, 0.0],
            [-18.0, 38.0, 0.0],
            [2.0, 38.0, 0.0],
        ],
        [0.0, 0.0, 1.0],
    ),
    (
        [
            // Cathedral ceiling
            [2.0, 6.0, 20.0],
            [-18.0, 6.0, 20.0],
            [-18.0, 38.0, 20.0],
            [2.0, 38.0, 20.0],
        ],
        [0.0, 0.0, 1.0],
    ),
    (
        [
            // Cathedral left wall 0
            [2.0, 6.0, 0.0],
            [-6.0, 6.0, 0.0],
            [-6.0, 6.0, 4.0],
            [2.0, 6.0, 4.0],
        ],
        [0.0, 1.0, 0.0],
    ),
    (
        [
            // Cathedral left wall 1
            [-10.0, 6.0, 0.0],
            [-18.0, 6.0, 0.0],
            [-18.0, 6.0, 4.0],
            [-10.0, 6.0, 4.0],
        ],
        [0.0, 1.0, 0.0],
    ),
    (
        [
            // Cathedral left wall upper
            [2.0, 6.0, 4.0],
            [-18.0, 6.0, 4.0],
            [-18.0, 6.0, 20.0],
            [2.0, 6.0, 20.0],
        ],
        [0.0, 1.0, 0.0],
    ),
    (
        [
            // Cathedral right wall
            [-18.0, 38.0, 0.0],
            [2.0, 38.0, 0.0],
            [2.0, 38.0, 20.0],
            [-18.0, 38.0, 20.0],
        ],
        [0.0, 1.0, 0.0],
    ),
    (
        [
            // Cathedral front wall
            [-18.0, 6.0, 0.0],
            [-18.0, 38.0, 0.0],
            [-18.0, 38.0, 20.0],
            [-18.0, 6.0, 20.0],
        ],
        [-1.0, 0.0, 0.0],
    ),
    (
        [
            // Cathedral back wall
            [2.0, 6.0, 0.0],
            [2.0, 38.0, 0.0],
            [2.0, 38.0, 20.0],
            [2.0, 6.0, 20.0],
        ],
        [-1.0, 0.0, 0.0],
    ),
];
