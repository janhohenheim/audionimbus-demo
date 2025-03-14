use bevy::prelude::*;

pub mod mouse_motion;
pub mod movement;

pub struct Plugin;

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(movement::Plugin);
        app.add_plugins(mouse_motion::Plugin);
    }
}
