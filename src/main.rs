use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, PrimitiveTopology},
    post_process::bloom::Bloom,
    prelude::*,
};
use bevy_seedling::prelude::*;

use crate::{
    audio::{
        AudionimbusContext, AudionimbusPool, AudionimbusReady, AudionimbusSimulator,
        AudionimbusSource,
    },
    camera_controller::CameraController,
};

mod audio;
mod camera_controller;
mod wrappers;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "audionimbus".to_string(),
                    mode: bevy::window::WindowMode::BorderlessFullscreen(MonitorSelection::Primary),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            SeedlingPlugin::default(),
        ))
        .add_plugins((audio::plugin, camera_controller::plugin))
        .add_observer(setup)
        .run();
}

fn setup(
    _ready: On<AudionimbusReady>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    context: Res<AudionimbusContext>,
    mut simulator: ResMut<AudionimbusSimulator>,
    assets: Res<AssetServer>,
) {
    let simulation_flags =
        audionimbus::SimulationFlags::DIRECT | audionimbus::SimulationFlags::REFLECTIONS;
    let source = audionimbus::Source::try_new(
        &simulator,
        &audionimbus::SourceSettings {
            flags: simulation_flags,
        },
    )
    .unwrap();
    simulator.add_source(&source);
    simulator.commit();

    let sphere = meshes.add(Sphere { radius: 0.1 });
    let sphere_material = materials.add(StandardMaterial {
        emissive: LinearRgba {
            red: 0.0,
            green: 0.0,
            blue: 1000.0,
            alpha: 1.0,
        },
        ..default()
    });

    #[cfg(not(any(feature = "direct", feature = "reverb")))]
    {
        let source_position = Transform::from_xyz(0.0, 2.0, 0.0);
        commands.spawn((
            Mesh3d(sphere.clone()),
            MeshMaterial3d(sphere_material.clone()),
            source_position,
            AudionimbusSource(source),
            GlobalTransform::default(),
            SamplePlayer::new(assets.load("selfless_courage.ogg")).looping(),
            AudionimbusPool,
        ));
        commands.spawn((
            source_position,
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
            Transform::from_xyz(28.0, 10.0, -8.0),
            PointLight {
                intensity: 5000000.0,
                color: Color::Srgba(Srgba {
                    red: 0.8,
                    green: 0.8,
                    blue: 1.0,
                    alpha: 1.0,
                }),
                ..Default::default()
            },
        ));
    }
    #[cfg(feature = "direct")]
    {
        let source_position = Transform::from_xyz(0.0, 2.0, 0.0);
        commands.spawn((
            Mesh3d(sphere.clone()),
            MeshMaterial3d(sphere_material.clone()),
            source_position,
            audio::AudioSource {
                source,
                data: samples,
                is_repeating: true,
                position: 0,
            },
        ));
        commands.spawn((
            source_position,
            PointLight {
                intensity: 500000.0,
                color: Color::Srgba(Srgba {
                    red: 0.8,
                    green: 0.8,
                    blue: 1.0,
                    alpha: 1.0,
                }),
                ..Default::default()
            },
        ));
    }
    #[cfg(feature = "reverb")]
    {
        let source_position = Transform::from_xyz(0.0, 8.0, -10.0);
        commands.spawn((
            Mesh3d(sphere.clone()),
            MeshMaterial3d(sphere_material.clone()),
            source_position,
            audio::AudioSource {
                source,
                data: samples,
                is_repeating: true,
                position: 0,
            },
        ));
        commands.spawn((
            source_position,
            PointLight {
                intensity: 30000000.0,
                color: Color::Srgba(Srgba {
                    red: 0.8,
                    green: 0.8,
                    blue: 1.0,
                    alpha: 1.0,
                }),
                ..Default::default()
            },
        ));
    }

    let mut scene =
        audionimbus::Scene::try_new(&context, &audionimbus::SceneSettings::default()).unwrap();

    let walls = audionimbus::StaticMesh::try_new(
        &scene,
        &audionimbus::StaticMeshSettings {
            vertices: &[
                // Floor
                audionimbus::Point::new(-2.0, 0.0, -2.0),
                audionimbus::Point::new(2.0, 0.0, -2.0),
                audionimbus::Point::new(2.0, 0.0, 2.0),
                audionimbus::Point::new(-2.0, 0.0, 2.0),
                // Ceiling
                audionimbus::Point::new(-2.0, 4.0, -2.0),
                audionimbus::Point::new(2.0, 4.0, -2.0),
                audionimbus::Point::new(2.0, 4.0, 2.0),
                audionimbus::Point::new(-2.0, 4.0, 2.0),
                // Back wall
                audionimbus::Point::new(-2.0, 0.0, -2.0),
                audionimbus::Point::new(2.0, 0.0, -2.0),
                audionimbus::Point::new(2.0, 4.0, -2.0),
                audionimbus::Point::new(-2.0, 4.0, -2.0),
                // Left wall
                audionimbus::Point::new(-2.0, 0.0, -2.0),
                audionimbus::Point::new(-2.0, 0.0, 2.0),
                audionimbus::Point::new(-2.0, 4.0, 2.0),
                audionimbus::Point::new(-2.0, 4.0, -2.0),
            ],
            triangles: &[
                // Floor
                audionimbus::Triangle::new(0, 1, 2),
                audionimbus::Triangle::new(0, 2, 3),
                // Ceiling
                audionimbus::Triangle::new(4, 6, 5),
                audionimbus::Triangle::new(4, 7, 6),
                // Back wall
                audionimbus::Triangle::new(8, 9, 10),
                audionimbus::Triangle::new(8, 10, 11),
                // Left wall
                audionimbus::Triangle::new(12, 14, 13),
                audionimbus::Triangle::new(12, 15, 14),
            ],
            material_indices: &[0, 0, 0, 0, 0, 0, 0, 0],
            materials: &[audionimbus::Material::WOOD],
        },
    )
    .unwrap();
    scene.add_static_mesh(&walls);

    for (vertices, normal) in TOPOLOGY {
        let normal = [normal[1], normal[2], normal[0]];
        commands.spawn((
            Mesh3d(
                meshes.add(
                    Mesh::new(
                        PrimitiveTopology::TriangleList,
                        RenderAssetUsages::default(),
                    )
                    .with_inserted_attribute(
                        Mesh::ATTRIBUTE_POSITION,
                        vertices
                            .iter()
                            .map(|vertex| [vertex[1], vertex[2], vertex[0]])
                            .collect::<Vec<_>>(),
                    )
                    .with_inserted_indices(Indices::U32(vec![0, 3, 1, 1, 3, 2]))
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
            &scene,
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
        scene.add_static_mesh(&surface);
    }
    scene.commit();

    simulator.set_scene(&scene);
    simulator.commit();

    commands.insert_resource(AmbientLight {
        brightness: 200.0,
        ..Default::default()
    });

    commands.spawn((
        CameraController::default(),
        Camera3d::default(),
        Bloom::NATURAL,
        Transform::from_xyz(-0.45, 2.17, 10.0),
    ));
}

// Blender vertex coordinates
#[cfg(not(any(feature = "direct", feature = "reverb")))]
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
#[cfg(feature = "direct")]
const TOPOLOGY: [([[f32; 3]; 4], [f32; 3]); 4] = [
    (
        [
            // Floor
            [2.0, -2.0, 0.0],
            [2.0, 2.0, 0.0],
            [-2.0, 2.0, 0.0],
            [-2.0, -2.0, 0.0],
        ],
        [0.0, 0.0, -1.0],
    ),
    (
        [
            // Ceiling
            [2.0, -2.0, 4.0],
            [2.0, 2.0, 4.0],
            [-2.0, 2.0, 4.0],
            [-2.0, -2.0, 4.0],
        ],
        [0.0, 0.0, -1.0],
    ),
    (
        [
            // Left wall
            [-2.0, -2.0, 0.0],
            [2.0, -2.0, 0.0],
            [2.0, -2.0, 4.0],
            [-2.0, -2.0, 4.0],
        ],
        [0.0, 1.0, 0.0],
    ),
    (
        [
            // Front wall
            [-2.0, -2.0, 0.0],
            [-2.0, 2.0, 0.0],
            [-2.0, 2.0, 4.0],
            [-2.0, -2.0, 4.0],
        ],
        [-1.0, 0.0, 0.0],
    ),
];
#[cfg(feature = "reverb")]
const TOPOLOGY: [([[f32; 3]; 4], [f32; 3]); 6] = [
    (
        [
            // Cathedral floor
            [20.0, -10.0, 0.0],
            [20.0, 10.0, 0.0],
            [-20.0, 10.0, 0.0],
            [-20.0, -10.0, 0.0],
        ],
        [0.0, 0.0, -1.0],
    ),
    (
        [
            // Cathedral ceiling
            [20.0, -10.0, 20.0],
            [20.0, 10.0, 20.0],
            [-20.0, 10.0, 20.0],
            [-20.0, -10.0, 20.0],
        ],
        [0.0, 0.0, -1.0],
    ),
    (
        [
            // Cathedral left wall
            [-20.0, -10.0, 0.0],
            [20.0, -10.0, 0.0],
            [20.0, -10.0, 20.0],
            [-20.0, -10.0, 20.0],
        ],
        [0.0, 1.0, 0.0],
    ),
    (
        [
            // Cathedral right wall
            [20.0, 10.0, 0.0],
            [-20.0, 10.0, 0.0],
            [-20.0, 10.0, 20.0],
            [20.0, 10.0, 20.0],
        ],
        [0.0, -1.0, 0.0],
    ),
    (
        [
            // Cathedral front wall
            [-20.0, 10.0, 0.0],
            [-20.0, -10.0, 0.0],
            [-20.0, -10.0, 20.0],
            [-20.0, 10.0, 20.0],
        ],
        [1.0, 0.0, 0.0],
    ),
    (
        [
            // Cathedral back wall
            [20.0, -10.0, 0.0],
            [20.0, 10.0, 0.0],
            [20.0, 10.0, 20.0],
            [20.0, -10.0, 20.0],
        ],
        [-1.0, 0.0, 0.0],
    ),
];
