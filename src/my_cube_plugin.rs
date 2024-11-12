use std::num::NonZero;

use bevy::{
    ecs::system::{
        lifetimeless::{SRes, SResMut},
        SystemState,
    },
    math::ivec2,
    prelude::*,
    render::{
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        render_asset::{RenderAssetUsages, RenderAssets},
        render_graph::{self, RenderGraph, RenderLabel},
        render_resource::{
            binding_types::texture_storage_2d, BindGroup, BindGroupEntries, BindGroupEntry,
            BindGroupLayout, BindGroupLayoutEntries, BindGroupLayoutEntry, BindingResource,
            BindingType, BufferBinding, BufferBindingType, BufferDescriptor,
            BufferSize, BufferUsages, CachedComputePipelineId, ComputePassDescriptor,
            ComputePipelineDescriptor, Extent3d, PipelineCache, ShaderStages, ShaderType,
            StorageBuffer, StorageTextureAccess, TextureDimension, TextureFormat, TextureUsages,
            UniformBuffer,
        },
        renderer::{RenderDevice, RenderQueue},
        texture::GpuImage,
        Render, RenderApp, RenderSet,
    },
};
use bytemuck::{Pod, Zeroable};

pub struct MyCubePlugin;

impl Plugin for MyCubePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, get_input);
    }
    fn finish(&self, app: &mut App) {
        app.add_plugins(ExtractResourcePlugin::<CubeImg>::default())
            .init_resource::<CubeImg>()
            .add_plugins(ExtractResourcePlugin::<InputStore>::default())
            .init_resource::<InputStore>()
            .add_systems(Update, switch_textures);

        let render_app = app.sub_app_mut(RenderApp);
        let sim_node = CubeSimNode::new(render_app.world_mut());
        render_app
            .add_systems(
                Render,
                prepare_bind_group.in_set(RenderSet::PrepareBindGroups),
            )
            .init_resource::<CubeRes>();

        let mut render_graph = render_app.world_mut().resource_mut::<RenderGraph>();
        render_graph.add_node(CubeLabel, sim_node);
        render_graph.add_node_edge(CubeLabel, bevy::render::graph::CameraDriverLabel);
        // do nothing
    }
}

// Switch texture to display every frame to show the one that was written to most recently.
fn switch_textures(images: Res<CubeImg>, mut displayed: Query<&mut Handle<Image>>) {
    let mut displayed = displayed.single_mut();
    *displayed = images.img.clone_weak();
}
fn get_input(
    mut input_store: ResMut<InputStore>,
    input: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
) {
    if input.pressed(KeyCode::KeyA) || input.pressed(KeyCode::ArrowLeft) {
        input_store.pos = -1;
        input_store.time_tick = time.elapsed_seconds();
    } else if input.pressed(KeyCode::KeyD) || input.pressed(KeyCode::ArrowRight) {
        input_store.pos = 1;
        input_store.time_tick = time.elapsed_seconds();
    }
}
#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct CubeLabel;
#[derive(Resource)]
struct CubeRes {
    update_sim: CachedComputePipelineId,
    update_cube: CachedComputePipelineId,
    update_check: CachedComputePipelineId,
    update_spawn: CachedComputePipelineId,
    bindgroup_sim: BindGroup,
    bindgroup_cube: BindGroup,
    bindgroup_index: BindGroup,
    bindgrouplayout_texture: BindGroupLayout,
    bindgroup_texture: Option<BindGroup>,
    buffer_index: StorageBuffer<Index>,
    buffer_index_pong: StorageBuffer<Index>,
    buffer_is_touch: StorageBuffer<Misc>,
}

impl FromWorld for CubeRes {
    fn from_world(world: &mut World) -> Self {
        let update_shader: Handle<Shader> = world.load_asset("cube.wgsl");
        let render_device = world.resource::<RenderDevice>();
        let pipeline_cache = world.resource::<PipelineCache>();
        let queue = world.resource::<RenderQueue>();

        //1
        let byte_size = 100 * 200 * 4;
        let buffer = render_device.create_buffer(&BufferDescriptor {
            label: Some("cube_date"),
            size: byte_size,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        //3
        let mut timer = StorageBuffer::<MyTimer>::default();
        {
            timer.write_buffer(&render_device, &queue);
        }
        let mut block = StorageBuffer::<Block>::default();
        {
            block.write_buffer(&render_device, &queue);
        }
        let mut is_touch = StorageBuffer::<Misc>::default();
        {
            is_touch.write_buffer(&render_device, &queue);
        }

        let buffer_layout = render_device.create_bind_group_layout(
            None,
            &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: Some(NonZero::new(80000).unwrap()),
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: Some(MyTimer::min_size()),
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: Some(Misc::min_size()),
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 3,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: Some(Block::min_size()),
                    },
                    count: None,
                },
            ],
        );
        let buffer_group = render_device.create_bind_group(
            None,
            &buffer_layout,
            &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::Buffer(BufferBinding {
                        buffer: &buffer,
                        offset: 0,
                        size: None,
                    }),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Buffer(BufferBinding {
                        buffer: timer.buffer().as_ref().unwrap(),
                        offset: 0,
                        size: None,
                    }),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::Buffer(BufferBinding {
                        buffer: is_touch.buffer().as_ref().unwrap(),
                        offset: 0,
                        size: None,
                    }),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::Buffer(BufferBinding {
                        buffer: block.buffer().as_ref().unwrap(),
                        offset: 0,
                        size: None,
                    }),
                },
            ],
        );

        //2
        let mut sims = UniformBuffer::<GpuSimParams>::default();
        {
            let mut x = GpuSimParams::default();
            x.time = 2.12313;
            sims.set(x);
            sims.write_buffer(&render_device, &queue);
        }
        let sims_layout = render_device.create_bind_group_layout(
            None,
            &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: Some(GpuSimParams::min_size()),
                },
                count: None,
            }],
        );
        let sims_group = render_device.create_bind_group(
            None,
            &sims_layout,
            &[BindGroupEntry {
                binding: 0,
                resource: BindingResource::Buffer(BufferBinding {
                    buffer: sims.buffer().as_ref().unwrap(),
                    offset: 0,
                    size: None,
                }),
            }],
        );

        let mut index_buffer: StorageBuffer<Index> = StorageBuffer::<Index>::default();
        {
            index_buffer.write_buffer(render_device, queue);
        }
        let mut index_buffer_pong: StorageBuffer<Index> = StorageBuffer::<Index>::default();
        {
            index_buffer_pong.write_buffer(render_device, queue);
        }
        let index_layout = render_device.create_bind_group_layout(
            None,
            &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: BufferSize::new(4),
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: BufferSize::new(4),
                    },
                    count: None,
                },
            ],
        );
        let index_group = render_device.create_bind_group(
            None,
            &index_layout,
            &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::Buffer(BufferBinding {
                        buffer: &index_buffer_pong.buffer().unwrap(),
                        offset: 0,
                        size: None,
                    }),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Buffer(BufferBinding {
                        buffer: &index_buffer.buffer().unwrap(),
                        offset: 0,
                        size: None,
                    }),
                },
            ],
        );

        let texture_bind_group_layout = render_device.create_bind_group_layout(
            "GameOfLifeImages",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::COMPUTE,
                (texture_storage_2d(
                    TextureFormat::Rgba32Float,
                    StorageTextureAccess::ReadWrite,
                ),),
            ),
        );
        let mut image = Image::new_fill(
            Extent3d {
                width: 100,
                height: 200,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            &[0, 0, 0, 255],
            TextureFormat::R32Float,
            RenderAssetUsages::RENDER_WORLD,
        );
        image.texture_descriptor.usage = TextureUsages::COPY_DST
            | TextureUsages::STORAGE_BINDING
            | TextureUsages::TEXTURE_BINDING;
        // let image0 = images.add(image.clone());
        // let image1 = images.add(image.clone());

        let update_pipeline = ComputePipelineDescriptor {
            label: None,
            layout: vec![
                sims_layout.clone(),
                buffer_layout.clone(),
                index_layout.clone(),
                texture_bind_group_layout.clone(),
            ],
            push_constant_ranges: vec![],
            shader: update_shader.clone(),
            shader_defs: vec![],
            entry_point: "main".into(),
        };
        let update_cube = ComputePipelineDescriptor {
            label: None,
            layout: vec![
                sims_layout.clone(),
                buffer_layout.clone(),
                index_layout.clone(),
                texture_bind_group_layout.clone(),
            ],
            push_constant_ranges: vec![],
            shader: update_shader.clone(),
            shader_defs: vec![],
            entry_point: "main_cube".into(),
        };
        let update_check = ComputePipelineDescriptor {
            label: None,
            layout: vec![
                sims_layout.clone(),
                buffer_layout.clone(),
                index_layout.clone(),
                texture_bind_group_layout.clone(),
            ],
            push_constant_ranges: vec![],
            shader: update_shader.clone(),
            shader_defs: vec![],
            entry_point: "push_in".into(),
        };
        let update_spawn = ComputePipelineDescriptor {
            label: None,
            layout: vec![
                sims_layout,
                buffer_layout,
                index_layout,
                texture_bind_group_layout.clone(),
            ],
            push_constant_ranges: vec![],
            shader: update_shader.clone(),
            shader_defs: vec![],
            entry_point: "check_and_spawn".into(),
        };
        //commands.insert_resource(CubeImg { img: image0 });
        CubeRes {
            update_sim: pipeline_cache.queue_compute_pipeline(update_pipeline),
            bindgroup_sim: sims_group,
            bindgroup_cube: buffer_group,
            bindgroup_index: index_group,
            bindgroup_texture: None,
            bindgrouplayout_texture: texture_bind_group_layout,
            buffer_index: index_buffer,

            buffer_index_pong: index_buffer_pong,
            update_cube: pipeline_cache.queue_compute_pipeline(update_cube),
            update_check: pipeline_cache.queue_compute_pipeline(update_check),
            update_spawn: pipeline_cache.queue_compute_pipeline(update_spawn),
            buffer_is_touch: is_touch,
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable, ShaderType)]
struct Block {
    shape: [i32; 16], // 4x4 方块形状（可以根据需要调整大小）
    position: IVec2,  // 当前方块的位置
    color: i32,       // 方块颜色
}
impl Default for Block {
    fn default() -> Self {
        Self {
            shape: [0, 0, 0, 0, 
                    1, 1, 1, 1, 
                    0, 0, 0, 0,
                    0, 0, 0, 0],
            position: ivec2(200, 30),
            color: 1,
        }
    }
}
#[repr(C)]
#[derive(Debug, Default, Copy, Clone, Pod, Zeroable, ShaderType)]
struct Misc {
    is_touch: i32,
    pos: i32,
    myseed: f32,
}
#[repr(C)]
#[derive(Debug, Default, Copy, Clone, Pod, Zeroable, ShaderType)]
pub struct Index {
    index: u32,
}
#[repr(C)]
#[derive(Debug, Default, Copy, Clone, Pod, Zeroable, ShaderType)]
pub struct GpuSimParams {
    /// Delta time, in seconds, since last effect system update.
    delta_time: f32,
    /// Current effect system simulation time since startup, in seconds.
    ///
    /// This is a lower-precision variant of [`SimParams::time`].
    time: f32,
    /// Virtual delta time, in seconds, since last effect system update.
    virtual_delta_time: f32,
    /// Current virtual time since startup, in seconds.
    ///
    /// This is a lower-precision variant of [`SimParams::time`].
    virtual_time: f32,
    /// Real delta time, in seconds, since last effect system update.
    real_delta_time: f32,
    /// Current real time since startup, in seconds.
    ///
    /// This is a lower-precision variant of [`SimParams::time`].
    real_time: f32,
    /// Total number of groups to simulate this frame. Used by the indirect
    /// compute pipeline to cap the compute thread to the actual number of
    /// groups to process.
    ///
    /// This is only used by the `vfx_indirect` compute shader.
    num_groups: u32,
    ping: u32,
}
#[repr(C)]
#[derive(Debug, Default, Copy, Clone, Pod, Zeroable, ShaderType)]
struct MyTimer {
    now_time: f32,
    max_time: f32,
}

fn prepare_bind_group(
    mut cube_res: ResMut<CubeRes>,
    gpu_images: Res<RenderAssets<GpuImage>>,
    game_of_life_images: Res<CubeImg>,
    render_device: Res<RenderDevice>,
) {
    match cube_res.bindgroup_texture {
        Some(_) => (),
        None => {
            let view_a = gpu_images.get(&game_of_life_images.img).unwrap();
            let bind_group_0 = render_device.create_bind_group(
                None,
                &cube_res.bindgrouplayout_texture,
                &BindGroupEntries::single(&view_a.texture_view),
            );
            cube_res.bindgroup_texture = Some(bind_group_0);
        }
    }
}
#[derive(Resource, Clone, ExtractResource)]
struct CubeImg {
    img: Handle<Image>,
}
#[derive(Resource, Default, Clone, ExtractResource)]
struct InputStore {
    pos: i32,
    time_tick: f32,
}

impl FromWorld for CubeImg {
    fn from_world(world: &mut World) -> Self {
        let mut images = world.resource_mut::<Assets<Image>>();
        let mut image = Image::new_fill(
            Extent3d {
                width: 100,
                height: 200,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            &[
                255, 0, 0, 255, 255, 0, 0, 255, 255, 0, 0, 255, 255, 0, 0, 255,
            ],
            TextureFormat::Rgba32Float,
            RenderAssetUsages::RENDER_WORLD,
        );
        image.texture_descriptor.usage = TextureUsages::COPY_DST
            | TextureUsages::STORAGE_BINDING
            | TextureUsages::TEXTURE_BINDING;
        let image0 = images.add(image.clone());
        CubeImg { img: image0 }
    }
}
//#[derive(Default)]
struct CubeSimNode {
    field1: f32,
    is_run: bool,
    system_state: SystemState<(
        SRes<RenderDevice>,
        SRes<RenderQueue>,
        SResMut<CubeRes>,
        SRes<InputStore>,
    )>,
}
impl CubeSimNode {
    pub fn new(world: &mut World) -> Self {
        Self {
            field1: 0.,
            is_run: false,
            system_state: SystemState::new(world),
        }
    }
}
impl render_graph::Node for CubeSimNode {
    fn update(&mut self, world: &mut World) {
        let Some(times) = world.get_resource::<Time>() else {
            eprintln!("render world has not time res");
            return;
        };
        self.field1 += times.delta_seconds();

        let time = times.elapsed_seconds();

        self.system_state.update_archetypes(world);
        let mut x = self.system_state.get_manual_mut(world);

        // let mut cube_res = world.resource_mut::<CubeRes>();
        // {

        // }

        if (self.field1) > 0.1 {
            self.field1 -= 0.1;
            self.is_run = true;
            //初始化 各个数据
            x.2.buffer_index.set(Index::default());
            x.2.buffer_index.write_buffer(&x.0, &x.1);
            x.2.buffer_index_pong.set(Index::default());
            x.2.buffer_index_pong.write_buffer(&x.0, &x.1);
            if x.3.time_tick + 0.1 > time {
                x.2.buffer_is_touch.set(Misc {
                    is_touch: 0,
                    pos: x.3.pos,
                    myseed: time,
                });
            } else {
                x.2.buffer_is_touch.set(Misc {
                    is_touch: 0,
                    pos: 0,
                    myseed: time,
                });
            }
            x.2.buffer_is_touch.write_buffer(&x.0, &x.1);
        } else {
            self.is_run = false;
        };
    }
    fn run<'w>(
        &self,
        _graph: &mut render_graph::RenderGraphContext,
        render_context: &mut bevy::render::renderer::RenderContext<'w>,
        world: &'w World,
    ) -> Result<(), render_graph::NodeRunError> {
        if !self.is_run {
            return Ok(());
        }

        println!("run sim");
        let cube_res = world.resource::<CubeRes>();
        let pipeline_cache = world.resource::<PipelineCache>();
        let update_pipe = pipeline_cache.get_compute_pipeline(cube_res.update_sim);
        let Some(update_pipe) = update_pipe else {
            return Ok(());
        };
        //for index in 0..200 {
            {
                let mut pass = render_context
                    .command_encoder()
                    .begin_compute_pass(&ComputePassDescriptor::default());
                pass.set_pipeline(update_pipe);
                pass.set_bind_group(0, &cube_res.bindgroup_sim, &[]);
                pass.set_bind_group(1, &cube_res.bindgroup_cube, &[]);
                pass.set_bind_group(2, &cube_res.bindgroup_index, &[]);
                // if (index % 2 == 0) {
                //     pass.set_bind_group(2, &cube_res.bindgroup_index, &[]);
                // } else {
                //     pass.set_bind_group(2, &cube_res.bindgroup_index_pong, &[]);
                // }
                pass.set_bind_group(3, &cube_res.bindgroup_texture.as_ref().unwrap(), &[]);
                pass.dispatch_workgroups(2, 1, 1);
            }
        //}
        let Some(update_cube_pipe) = pipeline_cache.get_compute_pipeline(cube_res.update_cube)
        else {
            return Ok(());
        };
        {
            let mut pass = render_context
                .command_encoder()
                .begin_compute_pass(&ComputePassDescriptor::default());

            pass.set_pipeline(update_cube_pipe);
            pass.set_bind_group(0, &cube_res.bindgroup_sim, &[]);
            pass.set_bind_group(1, &cube_res.bindgroup_cube, &[]);

            pass.set_bind_group(2, &cube_res.bindgroup_index, &[]);

            pass.set_bind_group(3, &cube_res.bindgroup_texture.as_ref().unwrap(), &[]);
            pass.dispatch_workgroups(1, 1, 1);
        }
        let Some(update_check_pipe) = pipeline_cache.get_compute_pipeline(cube_res.update_check)
        else {
            return Ok(());
        };
        {
            let mut pass = render_context
                .command_encoder()
                .begin_compute_pass(&ComputePassDescriptor::default());

            pass.set_pipeline(update_check_pipe);
            pass.set_bind_group(0, &cube_res.bindgroup_sim, &[]);
            pass.set_bind_group(1, &cube_res.bindgroup_cube, &[]);

            pass.set_bind_group(2, &cube_res.bindgroup_index, &[]);

            pass.set_bind_group(3, &cube_res.bindgroup_texture.as_ref().unwrap(), &[]);
            pass.dispatch_workgroups(1, 1, 1);
        }
        let Some(update_spawn_pipe) = pipeline_cache.get_compute_pipeline(cube_res.update_spawn)
        else {
            return Ok(());
        };
        {
            let mut pass = render_context
                .command_encoder()
                .begin_compute_pass(&ComputePassDescriptor::default());

            pass.set_pipeline(update_spawn_pipe);
            pass.set_bind_group(0, &cube_res.bindgroup_sim, &[]);
            pass.set_bind_group(1, &cube_res.bindgroup_cube, &[]);

            pass.set_bind_group(2, &cube_res.bindgroup_index, &[]);

            pass.set_bind_group(3, &cube_res.bindgroup_texture.as_ref().unwrap(), &[]);
            pass.dispatch_workgroups(1, 1, 1);
        }

        Ok(())
    }
}
