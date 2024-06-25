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

pub struct FlowLeniaComputePlugin;

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct FlowLeniaLabel;

impl Plugin for FlowLeniaComputePlugin {
    fn build(&self, app: &mut App) {
        // Extract the fluid_lenia image resource from the main world into the render world
        // for operation on by the compute shader and display on the sprite.
        app
            .add_systems(Startup, setup)
            .add_plugins(ExtractResourcePlugin::<FlowLeniaImage>::default());
        let render_app = app.sub_app_mut(RenderApp);
        render_app
            .add_systems(
            Render,
            prepare_bind_group.in_set(RenderSet::PrepareBindGroups),
        );

        let mut render_graph = render_app.world.resource_mut::<RenderGraph>();
        render_graph.add_node(FlowLeniaLabel, FlowLeniaNode::default());
        render_graph.add_node_edge(FlowLeniaLabel, bevy::render::graph::CameraDriverLabel,
        );
    }

    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app.init_resource::<FlowLeniaPipeline>();
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

    let mut growth_img = Image::new_fill(
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
    growth_img.texture_descriptor.usage =
        TextureUsages::COPY_DST | TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING;
    let growth_img = images.add(growth_img);

    commands.spawn(SpriteBundle {
        sprite: Sprite {
            custom_size: Some(Vec2::new(SIZE.0 as f32, SIZE.1 as f32)),
            ..default()
        },
        texture: color_img.clone(),
        // texture: growth_img.clone(),
        ..default()
    });
    commands.spawn(Camera2dBundle::default());
    commands.insert_resource(FlowLeniaImage{ color_img, growth_img });
}


#[derive(Resource, Clone, Deref, ExtractResource, AsBindGroup)]
struct FlowLeniaImage {
    #[deref]
    #[storage_texture(0, image_format = Rgba8Unorm, access = ReadWrite)]
    color_img: Handle<Image>,
    #[storage_texture(1, image_format = R32Float, access = ReadWrite)]
    growth_img: Handle<Image>,
}

#[derive(Resource)]
struct FlowLeniaImageBindGroup(BindGroup);

fn prepare_bind_group(
    mut commands: Commands,
    pipeline: Res<FlowLeniaPipeline>,
    gpu_images: Res<RenderAssets<Image>>,
    fluid_lenia_image: Res<FlowLeniaImage>,
    render_device: Res<RenderDevice>,
) {
    let color_view = gpu_images.get(&fluid_lenia_image.color_img).unwrap();
    let growth_view = gpu_images.get(&fluid_lenia_image.growth_img).unwrap();

    let bind_group = render_device.create_bind_group(
        None,
        &pipeline.texture_bind_group_layout,
        &BindGroupEntries::with_indices((
            (0, &color_view.texture_view),
            (1, &growth_view.texture_view),
        )),
    );
    commands.insert_resource(FlowLeniaImageBindGroup(bind_group));
}

#[derive(Resource)]
pub struct FlowLeniaPipeline {
    texture_bind_group_layout: BindGroupLayout,
    init_pipeline: CachedComputePipelineId,
    compute_growth_pipeline: CachedComputePipelineId,
    apply_flow_pipeline: CachedComputePipelineId,
}

// パイプラインの初期化を定義
impl FromWorld for FlowLeniaPipeline {
fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let texture_bind_group_layout = FlowLeniaImage::bind_group_layout(render_device);
        let shader = world
            .resource::<AssetServer>()
            .load("shaders/flow_lenia.compute.wgsl");
        let pipeline_cache = world.resource::<PipelineCache>();
        let init_pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: None,
            layout: vec![texture_bind_group_layout.clone()],
            push_constant_ranges: Vec::new(),
            shader: shader.clone(),
            shader_defs: vec![],
            entry_point: Cow::from("init"),
        });
        let compute_growth_pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: None,
            layout: vec![texture_bind_group_layout.clone()],
            push_constant_ranges: Vec::new(),
            shader: shader.clone(),
            shader_defs: vec![],
            entry_point: Cow::from("compute_growth"),
        });
        let apply_flow_pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: None,
            layout: vec![texture_bind_group_layout.clone()],
            push_constant_ranges: Vec::new(),
            shader,
            shader_defs: vec![],
            entry_point: Cow::from("apply_flow"),
        });

        FlowLeniaPipeline {
            texture_bind_group_layout,
            init_pipeline,
            compute_growth_pipeline,
            apply_flow_pipeline,
        }
    }
}

enum FlowLeniaState {
    Loading,
    Init,
    Update,
}

struct FlowLeniaNode {
    state: FlowLeniaState,
}

impl Default for FlowLeniaNode {
    fn default() -> Self {
        Self {
            state: FlowLeniaState::Loading,
        }
    }
}

impl render_graph::Node for FlowLeniaNode {
    fn update(&mut self, world: &mut World) {
        let pipeline = world.resource::<FlowLeniaPipeline>();
        let pipeline_cache = world.resource::<PipelineCache>();

        // if the corresponding pipeline has loaded, transition to the next stage
        match self.state {
            FlowLeniaState::Loading => {
                if let CachedPipelineState::Ok(_) =
                    pipeline_cache.get_compute_pipeline_state(pipeline.init_pipeline)
                {
                    self.state = FlowLeniaState::Init;
                }
            }
            FlowLeniaState::Init => {
                if let CachedPipelineState::Ok(_) =
                    pipeline_cache.get_compute_pipeline_state(pipeline.compute_growth_pipeline)
                {
                    self.state = FlowLeniaState::Update;
                }
            }
            FlowLeniaState::Update => {}
        }
    }

    fn run(
        &self,
        _graph: &mut render_graph::RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), render_graph::NodeRunError> {
        let texture_bind_group = &world.resource::<FlowLeniaImageBindGroup>().0;
        let pipeline_cache = world.resource::<PipelineCache>();
        let pipeline = world.resource::<FlowLeniaPipeline>();

        let mut pass = render_context
            .command_encoder()
            .begin_compute_pass(&ComputePassDescriptor::default());

        pass.set_bind_group(0, texture_bind_group, &[]);

        // select the pipeline based on the current state
        match self.state {
            FlowLeniaState::Loading => {}
            FlowLeniaState::Init => {
                let init_pipeline = pipeline_cache
                    .get_compute_pipeline(pipeline.init_pipeline)
                    .unwrap();
                pass.set_pipeline(init_pipeline);
                pass.dispatch_workgroups(SIZE.0 / WORKGROUP_SIZE, SIZE.1 / WORKGROUP_SIZE, 1);
            }
            FlowLeniaState::Update => {
                let compute_growth_pipeline = pipeline_cache
                    .get_compute_pipeline(pipeline.compute_growth_pipeline)
                    .unwrap();
                let apply_flow_pipeline = pipeline_cache
                    .get_compute_pipeline(pipeline.apply_flow_pipeline)
                    .unwrap();


                pass.set_pipeline(compute_growth_pipeline);
                pass.dispatch_workgroups(SIZE.0 / WORKGROUP_SIZE, SIZE.1 / WORKGROUP_SIZE, 1);
                pass.set_pipeline(apply_flow_pipeline);
                pass.dispatch_workgroups(SIZE.0 / WORKGROUP_SIZE, SIZE.1 / WORKGROUP_SIZE, 1);
            }
        }

        Ok(())
    }
}
