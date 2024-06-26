use bevy::{
    prelude::*,
    window::{Window, WindowPlugin},
};
mod ui;
mod lenia;
mod fluid;
mod flow_lenia;

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
                // lenia::LeniaComputePlugin,
                // fluid::FluidComputePlugin,
                flow_lenia::FlowLeniaComputePlugin,
        ))
        .run();
}

