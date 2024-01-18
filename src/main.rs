use bevy::{
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    prelude::*,
    render::{
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        render_asset::RenderAssets,
        render_graph::{self, RenderGraph},
        render_resource::*,
        renderer::{RenderContext, RenderDevice},
        Render, RenderApp, RenderSet,
    },
    window::WindowPlugin,
};
use std::borrow::Cow;

const SIZE: (u32, u32) = (600, 400);
const WORKGROUP_SIZE: u32 = 8;

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::BLACK))
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    // uncomment for unthrottled FPS
                    // present_mode: bevy::window::PresentMode::AutoNoVsync,
                    ..default()
                }),
                ..default()
            }),
            LeniaComputePlugin,
            FrameTimeDiagnosticsPlugin,
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, ui_system)
        .run();
}

fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    commands.spawn(TextBundle::from_section(
        "",
        TextStyle {
            font_size: 30.,
            color: Color::CYAN,
            ..default()
        },
    ));

    let mut image = Image::new_fill(
        Extent3d {
            width: SIZE.0,
            height: SIZE.1,
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
            custom_size: Some(Vec2::new(SIZE.0 as f32, SIZE.1 as f32)),
            ..default()
        },
        texture: image.clone(),
        ..default()
    });
    commands.spawn(Camera2dBundle::default());

    commands.insert_resource(LeniaImage(image));
}

pub struct LeniaComputePlugin;

impl Plugin for LeniaComputePlugin {
    fn build(&self, app: &mut App) {
        // Extract the lenia image resource from the main world into the render world
        // for operation on by the compute shader and display on the sprite.
        app.add_plugins(ExtractResourcePlugin::<LeniaImage>::default());
        let render_app = app.sub_app_mut(RenderApp);
        render_app.add_systems(
            Render,
            prepare_bind_group.in_set(RenderSet::PrepareBindGroups),
        );

        let mut render_graph = render_app.world.resource_mut::<RenderGraph>();
        render_graph.add_node("lenia", LeniaNode::default());
        render_graph.add_node_edge(
            "lenia",
            bevy::render::main_graph::node::CAMERA_DRIVER,
        );
    }

    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app.init_resource::<LeniaPipeline>();
    }
}

#[derive(Resource, Clone, Deref, ExtractResource)]
struct LeniaImage(Handle<Image>);

#[derive(Resource)]
struct LeniaImageBindGroup(BindGroup);

fn prepare_bind_group(
    mut commands: Commands,
    pipeline: Res<LeniaPipeline>,
    gpu_images: Res<RenderAssets<Image>>,
    lenia_image: Res<LeniaImage>,
    render_device: Res<RenderDevice>,
) {
    let view = gpu_images.get(&lenia_image.0).unwrap();
    let bind_group = render_device.create_bind_group(
        None,
        &pipeline.texture_bind_group_layout,
        &BindGroupEntries::single(&view.texture_view),
    );
    commands.insert_resource(LeniaImageBindGroup(bind_group));
}

#[derive(Resource)]
pub struct LeniaPipeline {
    texture_bind_group_layout: BindGroupLayout,
    init_pipeline: CachedComputePipelineId,
    update_pipeline: CachedComputePipelineId,
}

impl FromWorld for LeniaPipeline {
    fn from_world(world: &mut World) -> Self {
        let texture_bind_group_layout =
            world
                .resource::<RenderDevice>()
                .create_bind_group_layout(&BindGroupLayoutDescriptor {
                    label: None,
                    entries: &[BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::StorageTexture {
                            access: StorageTextureAccess::ReadWrite,
                            format: TextureFormat::Rgba8Unorm,
                            view_dimension: TextureViewDimension::D2,
                        },
                        count: None,
                    }],
                });
        let shader = world
            .resource::<AssetServer>()
            .load("shaders/lenia.wgsl");
        let pipeline_cache = world.resource::<PipelineCache>();
        let init_pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: None,
            layout: vec![texture_bind_group_layout.clone()],
            push_constant_ranges: Vec::new(),
            shader: shader.clone(),
            shader_defs: vec![],
            entry_point: Cow::from("init"),
        });
        let update_pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: None,
            layout: vec![texture_bind_group_layout.clone()],
            push_constant_ranges: Vec::new(),
            shader,
            shader_defs: vec![],
            entry_point: Cow::from("update"),
        });

        LeniaPipeline {
            texture_bind_group_layout,
            init_pipeline,
            update_pipeline,
        }
    }
}

enum LeniaState {
    Loading,
    Init,
    Update,
}

struct LeniaNode {
    state: LeniaState,
}

impl Default for LeniaNode {
    fn default() -> Self {
        Self {
            state: LeniaState::Loading,
        }
    }
}

impl render_graph::Node for LeniaNode {
    fn update(&mut self, world: &mut World) {
        let pipeline = world.resource::<LeniaPipeline>();
        let pipeline_cache = world.resource::<PipelineCache>();

        // if the corresponding pipeline has loaded, transition to the next stage
        match self.state {
            LeniaState::Loading => {
                if let CachedPipelineState::Ok(_) =
                    pipeline_cache.get_compute_pipeline_state(pipeline.init_pipeline)
                {
                    self.state = LeniaState::Init;
                }
            }
            LeniaState::Init => {
                if let CachedPipelineState::Ok(_) =
                    pipeline_cache.get_compute_pipeline_state(pipeline.update_pipeline)
                {
                    self.state = LeniaState::Update;
                }
            }
            LeniaState::Update => {}
        }
    }

    fn run(
        &self,
        _graph: &mut render_graph::RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), render_graph::NodeRunError> {
        let texture_bind_group = &world.resource::<LeniaImageBindGroup>().0;
        let pipeline_cache = world.resource::<PipelineCache>();
        let pipeline = world.resource::<LeniaPipeline>();

        let mut pass = render_context
            .command_encoder()
            .begin_compute_pass(&ComputePassDescriptor::default());

        pass.set_bind_group(0, texture_bind_group, &[]);

        // select the pipeline based on the current state
        match self.state {
            LeniaState::Loading => {}
            LeniaState::Init => {
                let init_pipeline = pipeline_cache
                    .get_compute_pipeline(pipeline.init_pipeline)
                    .unwrap();
                pass.set_pipeline(init_pipeline);
                pass.dispatch_workgroups(SIZE.0 / WORKGROUP_SIZE, SIZE.1 / WORKGROUP_SIZE, 1);
            }
            LeniaState::Update => {
                let update_pipeline = pipeline_cache
                    .get_compute_pipeline(pipeline.update_pipeline)
                    .unwrap();
                pass.set_pipeline(update_pipeline);
                pass.dispatch_workgroups(SIZE.0 / WORKGROUP_SIZE, SIZE.1 / WORKGROUP_SIZE, 1);
            }
        }

        Ok(())
    }
}

fn ui_system(mut query: Query<&mut Text>, diag: Res<DiagnosticsStore>) {
    let mut text = query.single_mut();

    let Some(fps) = diag
        .get(FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|fps| fps.smoothed())
    else {
        return;
    };

    text.sections[0].value = format!("FPS: {:.0}", fps,);
}
