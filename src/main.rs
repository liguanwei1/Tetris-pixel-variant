use bevy::prelude::*;
use my_cube_plugin::MyCubePlugin;
mod my_cube_plugin;

fn main() {
    let mut app = App::new();

    app.add_plugins(DefaultPlugins
        .set(ImagePlugin::default_nearest()))
    .add_plugins(MyCubePlugin)
    .add_systems(Startup, setup)
    .run();
}

fn setup(mut commands: Commands){
    commands.spawn(Camera2dBundle::default());

    commands.spawn(SpriteBundle {
        sprite: Sprite {
            custom_size: Some(Vec2::new(300.0,600.0)),
            flip_y:true,
            ..default()
        },
        transform: Transform::from_scale(Vec3::splat(1.0)),
        ..default()
    });
}