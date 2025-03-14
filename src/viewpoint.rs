use bevy::math::{EulerRot, Quat, Vec3};
use bevy::prelude::*;

use super::input;

#[derive(Component, Default, Debug)]
pub struct Viewpoint {
    pub translation: Vec3,
    pub rotation: Quat,
    pub pitch: f32,
    pub yaw: f32,
}

pub struct Plugin;

impl Plugin {
    fn update_viewpoint(
        mut query_characters: Query<&mut Viewpoint>,
        mut mouse_motion_delta_event_reader: EventReader<input::mouse_motion::Delta>,
    ) {
        let mouse_motion_delta =
            if let Some(mouse_motion_delta) = mouse_motion_delta_event_reader.read().last() {
                mouse_motion_delta
            } else {
                return;
            };

        for mut viewpoint in query_characters.iter_mut() {
            viewpoint.yaw = f32::rem_euclid(
                viewpoint.yaw + -mouse_motion_delta.0.x / 1000.0,
                2.0 * std::f32::consts::PI,
            );
            viewpoint.pitch = f32::clamp(
                viewpoint.pitch + -mouse_motion_delta.0.y / 1000.0,
                -std::f32::consts::FRAC_PI_2,
                std::f32::consts::FRAC_PI_2,
            );
            viewpoint.rotation = Quat::from_euler(
                EulerRot::YXZ,
                f32::rem_euclid(viewpoint.yaw, 2.0 * std::f32::consts::PI),
                f32::clamp(
                    viewpoint.pitch,
                    -std::f32::consts::FRAC_PI_2,
                    std::f32::consts::FRAC_PI_2,
                ),
                0.0,
            );
        }
    }
}

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, Self::update_viewpoint);
    }
}
