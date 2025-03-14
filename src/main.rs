use bevy::core_pipeline::{bloom::Bloom, tonemapping::Tonemapping};
use bevy::prelude::*;

mod controls;
mod cursor;
mod input;

fn main() {
    let mut app = App::new();

    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "audionimbus".to_string(),
            ..Default::default()
        }),
        ..Default::default()
    }))
    .add_plugins(cursor::Plugin)
    .add_plugins(controls::Plugin)
    .add_plugins(input::Plugin)
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
        Transform::from_xyz(2.0, 2.0, 2.0),
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

    commands.spawn((
        Camera3d::default(),
        Camera {
            hdr: true,
            ..default()
        },
        Tonemapping::TonyMcMapface,
        Bloom::NATURAL,
        Transform::from_xyz(0.0, 7., 14.0).looking_at(Vec3::new(0., 1., 0.), Vec3::Y),
    ));
}
