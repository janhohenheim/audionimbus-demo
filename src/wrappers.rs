use bevy::{math::Vec3, transform::components::Transform};
use firewheel::diff::{Diff, Patch, RealtimeClone};

#[derive(Debug, Clone, RealtimeClone, Diff, Patch, PartialEq)]
pub(crate) struct AudionimbusSimulationOutputs {
    pub(crate) direct: audionimbus::DirectEffectParams,
    pub(crate) reflections: audionimbus::ReflectionEffectParams,
}

impl From<audionimbus::SimulationOutputs> for AudionimbusSimulationOutputs {
    fn from(outputs: audionimbus::SimulationOutputs) -> Self {
        Self {
            direct: outputs.direct().into_inner(),
            reflections: outputs.reflections().into_inner(),
        }
    }
}

pub(crate) trait AudionimbusCoordinateSystemFromTransform {
    fn from_transform(transform: Transform) -> audionimbus::CoordinateSystem;
}

impl AudionimbusCoordinateSystemFromTransform for audionimbus::CoordinateSystem {
    fn from_transform(transform: Transform) -> Self {
        let listener_position = transform.translation;

        let listener_orientation_right = transform.right();
        let listener_orientation_up = transform.up();
        let listener_orientation_ahead = transform.forward();
        Self {
            right: audionimbus::Vector3::from_vec3(listener_orientation_right),
            up: audionimbus::Vector3::from_vec3(listener_orientation_up),
            ahead: audionimbus::Vector3::from_vec3(listener_orientation_ahead),
            origin: audionimbus::Vector3::from_vec3(listener_position),
        }
    }
}

pub(crate) trait AudionimbusVector3FromVec3 {
    fn from_vec3(vec3: impl Into<Vec3>) -> Self;
}

impl AudionimbusVector3FromVec3 for audionimbus::Vector3 {
    fn from_vec3(vec3: impl Into<Vec3>) -> Self {
        let vec3 = vec3.into();
        Self {
            x: vec3.x,
            y: vec3.y,
            z: vec3.z,
        }
    }
}
