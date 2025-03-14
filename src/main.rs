use bevy::prelude::PluginGroup;

fn main() {
    let mut app = bevy::app::App::new();

    app.add_plugins(bevy::DefaultPlugins.set(bevy::prelude::WindowPlugin {
        primary_window: Some(bevy::prelude::Window {
            title: "audionimbus".to_string(),
            ..Default::default()
        }),
        ..Default::default()
    }));

    app.run();
}
