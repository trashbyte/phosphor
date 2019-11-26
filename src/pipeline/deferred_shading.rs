use std::sync::Arc;

use vulkano::buffer::BufferUsage;
use vulkano::command_buffer::{AutoCommandBufferBuilder, AutoCommandBuffer, DynamicState};
use vulkano::descriptor::descriptor_set::PersistentDescriptorSet;
use vulkano::descriptor::DescriptorSet;
use vulkano::device::Queue;
use vulkano::framebuffer::{Framebuffer, FramebufferAbstract, RenderPass, RenderPassDesc, Subpass, RenderPassAbstract};
use vulkano::image::SwapchainImage;
use vulkano::pipeline::viewport::Viewport;
use vulkano::pipeline::{GraphicsPipeline, GraphicsPipelineAbstract};
use vulkano::sampler::{Sampler, Filter, SamplerAddressMode, MipmapMode};
use winit::Window;

use crate::cpu_pool::XallocCpuBufferPool;
use crate::geometry::{DeferredShadingVertex, VertexPositionUV};
use crate::pipeline::RenderPipelineAbstract;
use crate::renderer::RenderInfo;
use crate::renderpass::DeferredShadingRenderPass;
use crate::shader::deferred_shading as DeferredShadingShaders;
use crate::shader::skybox as SkyboxShaders;
use crate::buffer::CpuAccessibleBufferXalloc;
use cgmath::Matrix4;
use std::path::Path;


pub struct DeferredShadingRenderPipeline {
    skybox_pipeline: Arc<dyn GraphicsPipelineAbstract + Send + Sync>,
    voxel_shading_pipeline: Arc<dyn GraphicsPipelineAbstract + Send + Sync>,
    pub framebuffers: Option<Vec<Arc<dyn FramebufferAbstract + Send + Sync>>>,
    renderpass: Arc<RenderPass<DeferredShadingRenderPass>>,
    voxel_uniform_buffer_pool: XallocCpuBufferPool<DeferredShadingShaders::vertex::ty::InstanceData>,
    // TODO: texture bindings per material
    voxel_texture_descriptors: Arc<dyn DescriptorSet + Send + Sync>,
    skybox_vertex_buffer: Arc<CpuAccessibleBufferXalloc<[VertexPositionUV]>>,
    skybox_index_buffer: Arc<CpuAccessibleBufferXalloc<[u32]>>,
}


impl DeferredShadingRenderPipeline {
    pub fn new(info: &RenderInfo) -> Self {
        let renderpass = Arc::new(
            DeferredShadingRenderPass {}
                .build_render_pass(info.device.clone())
                .unwrap()
        );

        let skybox_pipeline = {
            let vs = SkyboxShaders::vertex::Shader::load(info.device.clone()).expect("failed to create shader module");
            let fs = SkyboxShaders::fragment::Shader::load(info.device.clone()).expect("failed to create shader module");

            Arc::new(GraphicsPipeline::start()
                .vertex_input_single_buffer::<VertexPositionUV>()
                .vertex_shader(vs.main_entry_point(), ())
                .triangle_list()
                .viewports_dynamic_scissors_irrelevant(1)
                .fragment_shader(fs.main_entry_point(), ())
                .depth_stencil_simple_depth()
                .render_pass(Subpass::from(renderpass.clone(), 0).unwrap())
                .build(info.device.clone())
                .unwrap())
        };

        let voxel_shading_pipeline = {
            let vs = DeferredShadingShaders::vertex::Shader::load(info.device.clone()).expect("failed to create shader module");
            let fs = DeferredShadingShaders::fragment::Shader::load(info.device.clone()).expect("failed to create shader module");

            Arc::new(GraphicsPipeline::start()
                .cull_mode_back()
                .vertex_input_single_buffer::<DeferredShadingVertex>()
                .vertex_shader(vs.main_entry_point(), ())
                .triangle_list()
                .viewports_dynamic_scissors_irrelevant(1)
                .fragment_shader(fs.main_entry_point(), ())
                .depth_stencil_simple_depth()
                .render_pass(Subpass::from(renderpass.clone(), 1).unwrap())
                .build(info.device.clone())
                .unwrap())
        };

        let (meshes, _) = tobj::load_obj(&Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/SkySphere.obj"))).unwrap();
        let skybox_mesh = &meshes[0].mesh;
        #[allow(unused_assignments)]
        let (mut u, mut v) = (0f32, 0f32);
        let mut skybox_uvs = Vec::new();
        for (i, uv) in skybox_mesh.texcoords.iter().enumerate() {
            match i % 2 {
                0 => { u = *uv; },
                1 => {
                    v = *uv;
                    skybox_uvs.push([ u, v ]);
                },
                _ => unreachable!()
            }
        }
        skybox_uvs.reverse();
        let mut skybox_verts = Vec::new();
        #[allow(unused_assignments)]
        let (mut x, mut y, mut z) = (0f32, 0f32, 0f32);
        for (i, p) in skybox_mesh.positions.iter().enumerate() {
            match i % 3 {
                0 => { x = *p; },
                1 => { y = *p; },
                2 => {
                    z = *p;
                    let uv =skybox_uvs.pop().unwrap();
                    skybox_verts.push(VertexPositionUV { position: [ x, y, z ], uv: [ uv[0], uv[1] ] });
                },
                _ => unreachable!()
            }
        }

        let skybox_vertex_buffer = CpuAccessibleBufferXalloc::<[VertexPositionUV]>::from_iter(
            info.device.clone(), BufferUsage::all(),
            skybox_verts.iter().cloned()).expect("failed to create buffer");
        let skybox_index_buffer = CpuAccessibleBufferXalloc::<[u32]>::from_iter(
            info.device.clone(), BufferUsage::all(),
            skybox_mesh.indices.iter().cloned()).expect("failed to create buffer");

        let linear_sampler = Sampler::new(info.device.clone(), Filter::Linear, Filter::Linear, MipmapMode::Linear,
            SamplerAddressMode::Repeat, SamplerAddressMode::Repeat, SamplerAddressMode::Repeat,
            0.0, 4.0, 0.0, 0.0).unwrap();

        let voxel_texture_descriptors = Arc::new(PersistentDescriptorSet::start(voxel_shading_pipeline.clone(), 0)
            .add_sampled_image(info.tex_registry.get("grass").unwrap().clone(), linear_sampler.clone()).unwrap()
            .add_sampled_image(info.tex_registry.get("test_normal").unwrap().clone(), linear_sampler.clone()).unwrap()
            .add_sampled_image(info.tex_registry.get("black").unwrap().clone(), linear_sampler.clone()).unwrap()
            .add_sampled_image(info.tex_registry.get("black").unwrap().clone(), linear_sampler.clone()).unwrap()
            .build().unwrap()
        );

        DeferredShadingRenderPipeline {
            skybox_pipeline,
            voxel_shading_pipeline,
            framebuffers: None,
            renderpass,
            voxel_uniform_buffer_pool: XallocCpuBufferPool::<DeferredShadingShaders::vertex::ty::InstanceData>::new(info.device.clone(), BufferUsage::all()),
            voxel_texture_descriptors,
            skybox_vertex_buffer,
            skybox_index_buffer,
        }
    }
}

const CLEAR_BLACK: [f32; 4] = [0.0, 0.0, 0.0, 1.0];

impl RenderPipelineAbstract for DeferredShadingRenderPipeline {
    fn get_framebuffers_mut(&mut self) -> &mut Option<Vec<Arc<dyn FramebufferAbstract + Send + Sync>>> {
        &mut self.framebuffers
    }


    fn get_renderpass(&self) -> Arc<dyn RenderPassAbstract + Send + Sync> {
        self.renderpass.clone() as Arc<dyn RenderPassAbstract + Send + Sync>
    }

    fn build_command_buffer(&mut self, info: &RenderInfo) -> (AutoCommandBuffer, Arc<Queue>) {
        let mut voxel_descriptor_sets = Vec::new();
        let lock = info.render_queues.read().unwrap();
        for entry in lock.meshes.iter() {
            let uniform_data = DeferredShadingShaders::vertex::ty::InstanceData {
                world: entry.transform.clone().into()
            };

            let subbuffer = self.voxel_uniform_buffer_pool.next(uniform_data).unwrap();
            voxel_descriptor_sets.push(Arc::new(PersistentDescriptorSet::start(self.voxel_shading_pipeline.clone(), 1)
                .add_buffer(subbuffer).unwrap()
                .build().unwrap()
            ));
        };

        let mut cb = AutoCommandBufferBuilder::primary_one_time_submit(info.device.clone(), info.queue_main.family())
            .unwrap()
            .begin_render_pass(self.framebuffers.as_ref().unwrap()[info.image_num].clone(), false,
                               vec![CLEAR_BLACK.into(), CLEAR_BLACK.into(), CLEAR_BLACK.into(), CLEAR_BLACK.into(), CLEAR_BLACK.into(), 1f32.into()]).unwrap()
                .draw_indexed(self.skybox_pipeline.clone(), &DynamicState {
                    line_width: None,
                    viewports: Some(vec![Viewport {
                        origin: [0.0, 0.0],
                        dimensions: [info.dimensions[0] as f32, info.dimensions[1] as f32],
                        depth_range: 0.0..1.0,
                    }]),
                    scissors: None,
                    compare_mask: None,
                    write_mask: None,
                    reference: None
                },
                              vec![self.skybox_vertex_buffer.clone()],
                              self.skybox_index_buffer.clone(),
                              (), SkyboxShaders::vertex::ty::Constants {
                                matrix: (info.proj_mat.clone() * Matrix4::from(info.camera_transform.rotation)).into(),
                                sun_rotation: 0.0,
                                sun_transit: 0.4,
                            }).unwrap()
            .next_subpass(false).unwrap();

        for (i, entry) in lock.meshes.iter().enumerate() {
            cb = cb.draw_indexed(self.voxel_shading_pipeline.clone(), &DynamicState {
                line_width: None,
                viewports: Some(vec![Viewport {
                    origin: [0.0, 0.0],
                    dimensions: [info.dimensions[0] as f32, info.dimensions[1] as f32],
                    depth_range: 0.0..1.0,
                }]),
                scissors: None,
                compare_mask: None,
                write_mask: None,
                reference: None
            },
                                 vec![entry.vertex_group.vertex_buffer.clone()],
                                 entry.vertex_group.index_buffer.clone(),
                                 (self.voxel_texture_descriptors.clone(), voxel_descriptor_sets[i].clone()),
                                 DeferredShadingShaders::vertex::ty::Constants {
                                     view: info.view_mat.into(),
                                     proj: info.proj_mat.into(),
                                 }).unwrap();
        }
        cb = cb.end_render_pass().unwrap();

        (cb.build().unwrap(), info.queue_main.clone())
    }

    fn recreate_framebuffers_if_none(&mut self, images: &Vec<Arc<SwapchainImage<Window>>>, info: &RenderInfo) {
        if self.get_framebuffers_mut().is_none() {
            let new_framebuffers = Some(images.iter().map(|_image| {
                let arc: Arc<dyn FramebufferAbstract + Send + Sync> = Arc::new(Framebuffer::start(self.get_renderpass().clone())
                    .add(info.attachments.position.clone()).unwrap()
                    .add(info.attachments.normal.clone()).unwrap()
                    .add(info.attachments.albedo.clone()).unwrap()
                    .add(info.attachments.roughness.clone()).unwrap()
                    .add(info.attachments.metallic.clone()).unwrap()
                    .add(info.attachments.main_depth.clone()).unwrap()
                    .build().unwrap());
                arc
            }).collect::<Vec<_>>());
            ::std::mem::replace(self.get_framebuffers_mut(), new_framebuffers);
        }
    }
}
