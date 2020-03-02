use std::sync::Arc;
use cgmath::Matrix4;
use vulkano::pipeline::{GraphicsPipeline, GraphicsPipelineAbstract};
use vulkano::framebuffer::{Framebuffer, FramebufferAbstract, RenderPassDesc, Subpass, RenderPassAbstract};
use vulkano::device::{Device, Queue};
use vulkano::sampler::{Sampler, Filter, MipmapMode, SamplerAddressMode};
use vulkano::buffer::BufferUsage;
use vulkano::command_buffer::{DynamicState, AutoCommandBufferBuilder, AutoCommandBuffer};
use vulkano::pipeline::viewport::Viewport;
use vulkano::image::SwapchainImage;
use winit::Window;

use crate::renderpass::GenericMeshShadingRenderPass;
use crate::cpu_pool::XallocCpuBufferPool;
use crate::geometry::MeshVertex;
use crate::shader::mesh_generic as MeshShaders;
use crate::stage::RenderStageDefinition;
use crate::renderer::RenderInfo;

pub struct GenericMeshShadingStage {
    pipeline: Arc<dyn GraphicsPipelineAbstract + Send + Sync>,
    pub framebuffers: Option<Vec<Arc<dyn FramebufferAbstract + Send + Sync>>>,
    pub framebuffer: Option<Arc<dyn FramebufferAbstract + Send + Sync>>,
    renderpass: Arc<dyn RenderPassAbstract + Send + Sync>,
    uniform_buffer_pool: XallocCpuBufferPool<MeshShaders::vertex::ty::InstanceData>,
}


impl GenericMeshShadingStage {
    pub fn new(device: Arc<Device>) -> Self {
        let renderpass = Arc::new(
            GenericMeshShadingRenderPass {}
                .build_render_pass(device.clone())
                .unwrap()
        );

        let pipeline = {
            let vs = crate::shader::skybox::vertex::Shader::load(device.clone()).expect("failed to create shader module");
            let fs = crate::shader::skybox::fragment::Shader::load(device.clone()).expect("failed to create shader module");

            Arc::new(GraphicsPipeline::start()
                .cull_mode_back()
                .vertex_input_single_buffer::<MeshVertex>()
                .vertex_shader(vs.main_entry_point(), ())
                .triangle_list()
                .viewports_dynamic_scissors_irrelevant(1)
                .fragment_shader(fs.main_entry_point(), ())
                //.depth_stencil_simple_depth()
                .render_pass(Subpass::from(renderpass.clone(), 0).unwrap())
                .build(device.clone())
                .unwrap())
        };

        GenericMeshShadingStage {
            pipeline,
            framebuffers: None,
            framebuffer: None,
            renderpass,
            uniform_buffer_pool: XallocCpuBufferPool::<MeshShaders::vertex::ty::InstanceData>::new(device.clone(), BufferUsage::all()),
        }
    }
}

const CLEAR_BLACK: [f32; 4] = [0.0, 0.0, 0.0, 1.0];

impl RenderStageDefinition for GenericMeshShadingStage {
    fn get_pipeline(&self) -> &Arc<dyn GraphicsPipelineAbstract + Send + Sync> { &self.pipeline }
    fn get_renderpass(&self) -> &Arc<dyn RenderPassAbstract + Send + Sync> { &self.renderpass }
    fn get_framebuffers(&self) -> &Option<Vec<Arc<dyn FramebufferAbstract + Send + Sync>>> { &self.framebuffers }
    fn get_framebuffers_mut(&mut self) -> &mut Option<Vec<Arc<dyn FramebufferAbstract + Send + Sync>>> { &mut self.framebuffers }

    fn build_command_buffers(&mut self, info: &RenderInfo) -> Option<Vec<(AutoCommandBuffer, Arc<Queue>)>> {
        let mut cb = AutoCommandBufferBuilder::primary_one_time_submit(info.device.clone(), info.queues.main.as_ref().unwrap().family())
            .unwrap()
            .begin_render_pass(self.framebuffer.as_ref().unwrap().clone(), false,
                               vec![CLEAR_BLACK.into(), CLEAR_BLACK.into(), CLEAR_BLACK.into(), CLEAR_BLACK.into(), CLEAR_BLACK.into(), 1f32.into()]).unwrap();

        let lock = info.mesh_queue.lock();
        for mesh in lock.iter() {
            for vertgroup in mesh.vertex_groups.iter() {
                cb = cb.draw_indexed(vertgroup.material.pipeline().clone(), &DynamicState {
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
                vec![vertgroup.vertex_buffer.clone()],
                vertgroup.index_buffer.clone(),
                vertgroup.material.descriptor_sets(),
                // TODO: handle actual push constants
                crate::shader::skybox::vertex::ty::Constants {
                    matrix: (info.proj_mat.clone() * Matrix4::from(info.camera_transform.rotation)).into(),
                    sun_rotation: 0.0,
                    sun_transit: 0.4,
                }).unwrap();
            }
        }
        cb = cb.end_render_pass().unwrap();

        Some(vec![
            (cb.build().unwrap(), info.queues.main.as_ref().unwrap().clone()),
        ])
    }

    fn recreate_framebuffers_if_none(&mut self, images: &Vec<Arc<SwapchainImage<Window>>>, info: &RenderInfo) {
        // TODO: framebuffer sets for standalone mode
//        if self.framebuffer.is_none() {
//            self.framebuffer = Some(Arc::new(Framebuffer::start(self.get_renderpass().clone())
//                .add(info.attachments.position.clone()).unwrap()
//                .add(info.attachments.normal.clone()).unwrap()
//                .add(info.attachments.albedo.clone()).unwrap()
//                .add(info.attachments.roughness.clone()).unwrap()
//                .add(info.attachments.metallic.clone()).unwrap()
//                .add(info.attachments.main_depth.clone()).unwrap()
//                .build().unwrap()))
//        }
//        if self.get_framebuffers_mut().is_none() {
//            let new_framebuffers = Some(images.iter().map(|_| {
//                let arc: Arc<dyn FramebufferAbstract + Send + Sync> = Arc::new(Framebuffer::start(self.get_renderpass().clone())
//                    .add(registry.get(name!(Position)).unwrap().as_floatbuf().unwrap().clone()).unwrap()
//                    .add(registry.get(name!(Albedo)).unwrap().as_floatbuf().unwrap().clone()).unwrap()
//                    .add(registry.get(name!(Normal)).unwrap().as_floatbuf().unwrap().clone()).unwrap()
//                    .add(registry.get(name!(Roughness)).unwrap().as_floatbuf().unwrap().clone()).unwrap()
//                    .add(registry.get(name!(Metallic)).unwrap().as_floatbuf().unwrap().clone()).unwrap()
//                    .add(registry.get(name!(MainDepth)).unwrap().as_depth().unwrap().clone()).unwrap()
//                    .build().unwrap());
//                arc
//            }).collect::<Vec<_>>());
//            ::std::mem::replace(self.get_framebuffers_mut(), new_framebuffers);
//        }
    }
}
