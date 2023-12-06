use bevy::prelude::*;
use megadodge_mayhem::GamePlugin;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    fit_canvas_to_parent: true,
                    ..default()
                }),
                ..default()
            }),
            GamePlugin,
        ))
        .run();
}
