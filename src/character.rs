use bevy::ecs::event::EventReader;
use bevy::prelude::*;
use bevy_rapier3d::plugin::*;

use super::input;
use super::viewpoint;

#[derive(Bundle, Default)]
pub struct Character {
    pub character_marker: CharacterMarker,
    pub visibility: Visibility,
    pub inherited_visibility: InheritedVisibility,
    pub view_visibility: ViewVisibility,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub viewpoint: viewpoint::Viewpoint,
    pub rigid_body: bevy_rapier3d::dynamics::RigidBody,
    pub collider: bevy_rapier3d::geometry::Collider,
    pub external_force: bevy_rapier3d::dynamics::ExternalForce,
    pub external_impulse: bevy_rapier3d::dynamics::ExternalImpulse,
    pub kinematics: Kinematics,
}

#[derive(Component, Default, Debug)]
pub struct CharacterMarker;

#[derive(Component, Debug)]
pub struct Kinematics {
    pub acceleration: Vec3,
    pub velocity: Vec3,
    pub ground_speed: f32,
    pub action: Action,
}

#[derive(Default, Debug)]
pub struct Action {
    pub forward: f32,
    pub strafe: f32,
}

impl std::default::Default for Kinematics {
    fn default() -> Self {
        Self {
            acceleration: Vec3::default(),
            velocity: Vec3::default(),
            ground_speed: 3.0,
            action: Action {
                forward: 0.0,
                strafe: 0.0,
            },
        }
    }
}

pub struct Plugin;

impl Plugin {
    fn read_movement_inputs(
        mut query_characters: Query<&mut Kinematics, With<CharacterMarker>>,
        mut movement_input_event_reader: EventReader<input::movement::Input>,
    ) {
        let movement_input_event = movement_input_event_reader.read().last();

        for mut kinematics in query_characters.iter_mut() {
            if let Some(movement_input) = movement_input_event {
                // Maps the movement coordinates to a circle (from a square).
                kinematics.action.forward =
                    movement_input.forward * f32::sqrt(1.0 - movement_input.strafe.powi(2) / 2.0);
                kinematics.action.strafe =
                    movement_input.strafe * f32::sqrt(1.0 - movement_input.forward.powi(2) / 2.0);
            }
        }
    }

    fn update_kinematics(
        mut query_characters: Query<
            (
                Entity,
                &mut Transform,
                &bevy_rapier3d::geometry::Collider,
                &mut Kinematics,
                &viewpoint::Viewpoint,
            ),
            With<CharacterMarker>,
        >,
        rapier_context: ReadRapierContext,
        time: Res<Time>,
    ) {
        const CAST_OPTIONS: bevy_rapier3d::geometry::ShapeCastOptions =
            bevy_rapier3d::geometry::ShapeCastOptions {
                max_time_of_impact: 0.0,
                target_distance: 0.0,
                stop_at_penetration: false,
                compute_impact_geometry_on_penetration: false,
            };
        const IDLE_VELOCITY: f32 = 0.1;
        const UP: Vec3 = bevy::math::Vec3::Y;

        let time_delta = time.delta_secs();
        let rapier_context = rapier_context.single();

        for (entity, mut transform, collider, mut kinematics, viewpoint) in
            query_characters.iter_mut()
        {
            let (y_rotation, _, _) = viewpoint.rotation.to_euler(bevy::math::EulerRot::YXZ);
            let rotation = Quat::from_euler(EulerRot::YXZ, y_rotation, 0.0, 0.0);
            let forward = rotation * -Dir3::Z;
            let right = rotation * Dir3::X;

            let query_filter = bevy_rapier3d::pipeline::QueryFilter {
                exclude_rigid_body: Some(entity),
                flags: bevy_rapier3d::pipeline::QueryFilterFlags::ONLY_FIXED,
                ..Default::default()
            };

            let input_movement =
                forward * kinematics.action.forward + right * kinematics.action.strafe;

            let input_movement_velocity = input_movement * kinematics.ground_speed;
            kinematics.velocity = input_movement_velocity - input_movement_velocity.dot(UP) * UP;

            // Hack to prevent jittering when character is idle.
            if kinematics.velocity.length() < IDLE_VELOCITY {
                // Character is idle.
                kinematics.velocity = Vec3::ZERO;
                continue;
            }

            let forward_cast_result = rapier_context.cast_shape(
                transform.translation,
                transform.rotation,
                kinematics.velocity,
                collider,
                bevy_rapier3d::geometry::ShapeCastOptions {
                    max_time_of_impact: time_delta,
                    ..CAST_OPTIONS
                },
                query_filter,
            );
            if forward_cast_result.is_none() {
                // The rest of the path is unobstructed.
                transform.translation += kinematics.velocity * time_delta;
            }
        }
    }

    fn update_camera(
        mut query_cameras: Query<(&Parent, &mut Transform), With<Camera>>,
        query_viewpoint: Query<&viewpoint::Viewpoint>,
    ) {
        for (parent, mut transform) in query_cameras.iter_mut() {
            let viewpoint = query_viewpoint.get(parent.get()).unwrap();
            transform.translation = viewpoint.translation;
            transform.rotation = viewpoint.rotation;
        }
    }
}

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            (
                Self::read_movement_inputs,
                Self::update_kinematics,
                Self::update_camera,
            ),
        );
    }
}
