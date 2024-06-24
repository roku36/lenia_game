use bevy::{
    prelude::*,
    render::{
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        render_asset::RenderAssetUsages,
        render_asset::RenderAssets,
        render_graph::{self, RenderGraph, RenderLabel},
        render_resource::*,
        renderer::{RenderContext, RenderDevice},
        Render, RenderApp, RenderSet,
    },
};

pub const SIZE: (u32, u32) = (600, 400);
const WORKGROUP_SIZE: u32 = 8;
use std::borrow::Cow;

pub struct FluidComputePlugin;

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct FluidLabel;

impl Plugin for FluidComputePlugin {
    fn build(&self, app: &mut App) {
        // Extract the fluid image resource from the main world into the render world
        // for operation on by the compute shader and display on the sprite.
        app
            .add_systems(Startup, setup)
            .add_plugins(ExtractResourcePlugin::<FluidImage>::default());
        let render_app = app.sub_app_mut(RenderApp);
        render_app
            .add_systems(
            Render,
            prepare_bind_group.in_set(RenderSet::PrepareBindGroups),
        );

        let mut render_graph = render_app.world.resource_mut::<RenderGraph>();
        render_graph.add_node(FluidLabel, FluidNode::default());
        render_graph.add_node_edge(FluidLabel, bevy::render::graph::CameraDriverLabel,
        );
    }

    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app.init_resource::<FluidPipeline>();
    }
}

fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let mut color_img = Image::new_fill(
        Extent3d {
            width: SIZE.0,
            height: SIZE.1,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::RENDER_WORLD,
    );
    color_img.texture_descriptor.usage =
        TextureUsages::COPY_DST | TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING;
    let color_img = images.add(color_img);

    let mut velocity_x_img = Image::new_fill(
        Extent3d {
            width: SIZE.0,
            height: SIZE.1,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 0],
        TextureFormat::R32Float,
        RenderAssetUsages::RENDER_WORLD,
    );
    velocity_x_img.texture_descriptor.usage =
        TextureUsages::COPY_DST | TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING;
    let velocity_x_img = images.add(velocity_x_img);

    let mut velocity_y_img = Image::new_fill(
        Extent3d {
            width: SIZE.0,
            height: SIZE.1,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 0],
        TextureFormat::R32Float,
        RenderAssetUsages::RENDER_WORLD,
    );
    velocity_y_img.texture_descriptor.usage =
        TextureUsages::COPY_DST | TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING;
    let velocity_y_img = images.add(velocity_y_img);

    let mut pressure_img = Image::new_fill(
        Extent3d {
            width: SIZE.0,
            height: SIZE.1,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 0],
        TextureFormat::R32Float,
        RenderAssetUsages::RENDER_WORLD,
    );
    pressure_img.texture_descriptor.usage =
        TextureUsages::COPY_DST | TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING;
    let pressure_img = images.add(pressure_img);

    commands.spawn(SpriteBundle {
        sprite: Sprite {
            custom_size: Some(Vec2::new(SIZE.0 as f32, SIZE.1 as f32)),
            ..default()
        },
        texture: color_img.clone(),
        // texture: velocity_x_img.clone(),
        // texture: pressure_img.clone(),
        ..default()
    });
    commands.spawn(Camera2dBundle::default());
    commands.insert_resource(FluidImage{ color_img, velocity_x_img, velocity_y_img, pressure_img });
}


#[derive(Resource, Clone, Deref, ExtractResource, AsBindGroup)]
struct FluidImage {
    #[deref]
    #[storage_texture(0, image_format = Rgba8Unorm, access = ReadWrite)]
    color_img: Handle<Image>,
    #[storage_texture(1, image_format = R32Float, access = ReadWrite)]
    velocity_x_img: Handle<Image>,
    #[storage_texture(2, image_format = R32Float, access = ReadWrite)]
    velocity_y_img: Handle<Image>,
    #[storage_texture(3, image_format = R32Float, access = ReadWrite)]
    pressure_img: Handle<Image>,
}

#[derive(Resource)]
struct FluidImageBindGroup(BindGroup);

fn prepare_bind_group(
    mut commands: Commands,
    pipeline: Res<FluidPipeline>,
    gpu_images: Res<RenderAssets<Image>>,
    fluid_image: Res<FluidImage>,
    render_device: Res<RenderDevice>,
) {
    let color_view = gpu_images.get(&fluid_image.color_img).unwrap();
    let velocity_x_view = gpu_images.get(&fluid_image.velocity_x_img).unwrap();
    let velocity_y_view = gpu_images.get(&fluid_image.velocity_y_img).unwrap();
    let pressure_view = gpu_images.get(&fluid_image.pressure_img).unwrap();

    let bind_group = render_device.create_bind_group(
        None,
        &pipeline.texture_bind_group_layout,
        &BindGroupEntries::with_indices((
            (0, &color_view.texture_view),
            (1, &velocity_x_view.texture_view),
            (2, &velocity_y_view.texture_view),
            (3, &pressure_view.texture_view),
        )),
    );
    commands.insert_resource(FluidImageBindGroup(bind_group));
}

#[derive(Resource)]
pub struct FluidPipeline {
    texture_bind_group_layout: BindGroupLayout,
    init_pipeline: CachedComputePipelineId,
    update_pressure_pipeline: CachedComputePipelineId,
    update_pipeline: CachedComputePipelineId,
    // render_pipeline: CachedRenderPipelineId,
}

// パイプラインの初期化を定義
impl FromWorld for FluidPipeline {
fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let texture_bind_group_layout = FluidImage::bind_group_layout(render_device);
        let shader = world
            .resource::<AssetServer>()
            .load("shaders/fluid.compute.wgsl");
        let pipeline_cache = world.resource::<PipelineCache>();
        let init_pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: None,
            layout: vec![texture_bind_group_layout.clone()],
            push_constant_ranges: Vec::new(),
            shader: shader.clone(),
            shader_defs: vec![],
            entry_point: Cow::from("init"),
        });
        let update_pressure_pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: None,
            layout: vec![texture_bind_group_layout.clone()],
            push_constant_ranges: Vec::new(),
            shader: shader.clone(),
            shader_defs: vec![],
            entry_point: Cow::from("update_pressure"),
        });
        let update_pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: None,
            layout: vec![texture_bind_group_layout.clone()],
            push_constant_ranges: Vec::new(),
            shader,
            shader_defs: vec![],
            entry_point: Cow::from("update"),
        });

        FluidPipeline {
            texture_bind_group_layout,
            init_pipeline,
            update_pressure_pipeline,
            update_pipeline,
            // render_pipeline,
        }
    }
}

enum FluidState {
    Loading,
    Init,
    Update,
}

struct FluidNode {
    state: FluidState,
}

impl Default for FluidNode {
    fn default() -> Self {
        Self {
            state: FluidState::Loading,
        }
    }
}

impl render_graph::Node for FluidNode {
    fn update(&mut self, world: &mut World) {
        let pipeline = world.resource::<FluidPipeline>();
        let pipeline_cache = world.resource::<PipelineCache>();

        // if the corresponding pipeline has loaded, transition to the next stage
        match self.state {
            FluidState::Loading => {
                if let CachedPipelineState::Ok(_) =
                    pipeline_cache.get_compute_pipeline_state(pipeline.init_pipeline)
                {
                    self.state = FluidState::Init;
                }
            }
            FluidState::Init => {
                if let CachedPipelineState::Ok(_) =
                    pipeline_cache.get_compute_pipeline_state(pipeline.update_pipeline)
                {
                    self.state = FluidState::Update;
                }
            }
            FluidState::Update => {}
        }
    }

    fn run(
        &self,
        _graph: &mut render_graph::RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), render_graph::NodeRunError> {
        let texture_bind_group = &world.resource::<FluidImageBindGroup>().0;
        let pipeline_cache = world.resource::<PipelineCache>();
        let pipeline = world.resource::<FluidPipeline>();

        let mut pass = render_context
            .command_encoder()
            .begin_compute_pass(&ComputePassDescriptor::default());

        pass.set_bind_group(0, texture_bind_group, &[]);

        // select the pipeline based on the current state
        match self.state {
            FluidState::Loading => {}
            FluidState::Init => {
                let init_pipeline = pipeline_cache
                    .get_compute_pipeline(pipeline.init_pipeline)
                    .unwrap();
                pass.set_pipeline(init_pipeline);
                pass.dispatch_workgroups(SIZE.0 / WORKGROUP_SIZE, SIZE.1 / WORKGROUP_SIZE, 1);
            }
            FluidState::Update => {
                let update_pressure_pipeline = pipeline_cache
                    .get_compute_pipeline(pipeline.update_pressure_pipeline)
                    .unwrap();
                let update_pipeline = pipeline_cache
                    .get_compute_pipeline(pipeline.update_pipeline)
                    .unwrap();


                pass.set_pipeline(update_pipeline);
                pass.dispatch_workgroups(SIZE.0 / WORKGROUP_SIZE, SIZE.1 / WORKGROUP_SIZE, 1);
                pass.set_pipeline(update_pressure_pipeline);
                for _ in 0..100{
                    pass.dispatch_workgroups(SIZE.0 / WORKGROUP_SIZE, SIZE.1 / WORKGROUP_SIZE, 1);
                }
            }
        }

        Ok(())
    }
}
