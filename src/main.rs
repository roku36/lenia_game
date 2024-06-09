use bevy::{
    prelude::*,
    render::render_resource::*,
    window::{Window, WindowPlugin},
};
mod ui;
mod lenia;

use crate::ui::fps::FpsPlugin;

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::NONE))
        .add_plugins((
                DefaultPlugins.set(WindowPlugin {
                    primary_window: Some(Window {
                        // transparent: true,
                        // composite_alpha_mode: CompositeAlphaMode::PostMultiplied,
                        // decorations: false,
                        // uncomment for unthrottled FPS
                        // present_mode: bevy::window::PresentMode::AutoNoVsync,
                        ..default()
                    }),
                    ..default()
                }),
                FpsPlugin,
                lenia::LeniaComputePlugin,
        ))
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let mut image = Image::new_fill(
        Extent3d {
            width: lenia::SIZE.0,
            height: lenia::SIZE.1,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::Rgba8Unorm,
    );
    image.texture_descriptor.usage =
        TextureUsages::COPY_DST | TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING;
    let image = images.add(image);

    commands.spawn(SpriteBundle {
        sprite: Sprite {
            custom_size: Some(Vec2::new(lenia::SIZE.0 as f32, lenia::SIZE.1 as f32)),
            ..default()
        },
        texture: image.clone(),
        ..default()
    });
    commands.spawn(Camera2dBundle::default());
    commands.insert_resource(lenia::LeniaImage(image));
}

