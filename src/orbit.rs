use bevy::prelude::*;

#[derive(Component, Debug)]
pub struct Orbit {
    pub center: Vec3, // The center point of orbit.
    pub radius: f32,  // Radius of the orbit.
    pub angle: f32,   // Current angle in radians.
    pub speed: f32,   // Angular velocity (radians per second).
}

pub struct Plugin;

impl Plugin {
    fn tick_orbit(time: Res<Time>, mut query: Query<(&mut Transform, &mut Orbit)>) {
        for (mut transform, mut orbit) in query.iter_mut() {
            // Update angle based on speed
            orbit.angle += orbit.speed * time.delta_secs();

            // Compute new position
            transform.translation = Vec3::new(
                orbit.center.x + orbit.radius * orbit.angle.cos(),
                orbit.center.y,
                orbit.center.z + orbit.radius * orbit.angle.sin(),
            );
        }
    }
}

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, Self::tick_orbit);
    }
}
