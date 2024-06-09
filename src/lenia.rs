use bevy::{
    prelude::*,
    render::{
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        render_asset::RenderAssets,
        render_graph::{self, RenderGraph},
        render_resource::*,
        renderer::{RenderContext, RenderDevice},
        Render, RenderApp, RenderSet,
    },
};
use std::borrow::Cow;
pub struct LeniaComputePlugin;

pub const SIZE: (u32, u32) = (600, 400);
const WORKGROUP_SIZE: u32 = 8;

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
pub struct LeniaImage(pub Handle<Image>);

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
    // render_pipeline: CachedComputePipelineId,
}

impl FromWorld for LeniaPipeline {
    fn from_world(world: &mut World) -> Self {
        let texture_bind_group_layout =
        world
            .resource::<RenderDevice>()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: None,
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::StorageTexture {
                            access: StorageTextureAccess::ReadWrite,
                            format: TextureFormat::Rgba8Unorm,
                            view_dimension: TextureViewDimension::D2,
                        },
                        count: None,
                    },
                    // BindGroupLayoutEntry {
                    //     binding: 1,
                    //     visibility: ShaderStages::FRAGMENT,
                    //     ty: BindingType::StorageTexture {
                    //         access: StorageTextureAccess::ReadOnly,
                    //         format: TextureFormat::Rgba8Unorm,
                    //         view_dimension: TextureViewDimension::D2,
                    //     },
                    //     count: None,
                    // },
                ],
            });
        let shader: Handle<Shader> = world
            .resource::<AssetServer>()
            .load("shaders/lenia.compute.wgsl");
        // let color_shader: Handle<Shader> = world
        //     .resource::<AssetServer>()
        //     .load("shaders/gradient.wgsl");
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
        // let render_pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
        //     label: None,
        //     layout: vec![texture_bind_group_layout.clone()],
        //     push_constant_ranges: Vec::new(),
        //     shader: color_shader,
        //     shader_defs: vec![],
        //     entry_point: Cow::from("render"),
        // });

        LeniaPipeline {
            texture_bind_group_layout,
            init_pipeline,
            update_pipeline,
            // render_pipeline,
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
