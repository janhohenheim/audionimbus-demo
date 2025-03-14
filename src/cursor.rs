use bevy::prelude::*;
use bevy::window::{CursorGrabMode, PrimaryWindow};

pub struct Plugin;

impl Plugin {
    fn grab_cursor(mut query: Query<&mut Window, With<PrimaryWindow>>) {
        let mut primary_window = query.single_mut();
        primary_window.cursor_options.grab_mode = CursorGrabMode::Locked;
        primary_window.cursor_options.visible = false;
    }

    fn ungrab_cursor(
        mut query: bevy::ecs::prelude::Query<
            &mut bevy::prelude::Window,
            bevy::ecs::prelude::With<PrimaryWindow>,
        >,
    ) {
        let mut primary_window = query.single_mut();
        primary_window.cursor_options.grab_mode = CursorGrabMode::None;
        primary_window.cursor_options.visible = true;
    }
}

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, Self::grab_cursor);
    }
}
