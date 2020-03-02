use std::sync::Arc;
use vulkano::pipeline::{GraphicsPipeline, GraphicsPipelineAbstract};
use vulkano::framebuffer::{Framebuffer, FramebufferAbstract, RenderPassDesc, Subpass, RenderPassAbstract};
use vulkano::descriptor::DescriptorSet;
use vulkano::device::{Device, Queue};
use vulkano::buffer::BufferUsage;
use vulkano::command_buffer::{DynamicState, AutoCommandBufferBuilder, AutoCommandBuffer};
use vulkano::descriptor::descriptor_set::PersistentDescriptorSet;
use vulkano::pipeline::viewport::Viewport;
use vulkano::image::{SwapchainImage, AttachmentImage};
use vulkano::format::{ClearValue, R16G16B16A16Sfloat, R32Uint};
use winit::Window;

use crate::renderpass::ResolveSceneColorRenderPass;
use crate::buffer::CpuAccessibleBufferXalloc;
use crate::geometry::VertexPosition;
use crate::shader::resolve_scene_color as ResolveShaders;
use crate::stage::RenderStageDefinition;
use crate::renderer::RenderInfo;

pub struct ResolveSceneColorStage {
    pipeline: Arc<dyn GraphicsPipelineAbstract + Send + Sync>,
    pub framebuffers: Option<Vec<Arc<dyn FramebufferAbstract + Send + Sync>>>,
    pub framebuffer: Option<Arc<dyn FramebufferAbstract + Send + Sync>>,
    renderpass: Arc<dyn RenderPassAbstract + Send + Sync>,
    fullscreen_vertex_buffer: Arc<CpuAccessibleBufferXalloc<[VertexPosition]>>,
    descriptor_set: Arc<dyn DescriptorSet + Send + Sync>,
}


impl ResolveSceneColorStage {
    pub fn new(device: Arc<Device>, scene_color: Arc<AttachmentImage<R16G16B16A16Sfloat>>, luma_out: Arc<AttachmentImage<R32Uint>>) -> Self {
        let renderpass = Arc::new(
            ResolveSceneColorRenderPass {}
                .build_render_pass(device.clone())
                .unwrap()
        );

        let pipeline = {
            let vs = ResolveShaders::vertex::Shader::load(device.clone()).expect("failed to create shader module");
            let fs = ResolveShaders::fragment::Shader::load(device.clone()).expect("failed to create shader module");

            Arc::new(GraphicsPipeline::start()
                .cull_mode_back()
                .vertex_input_single_buffer::<VertexPosition>()
                .vertex_shader(vs.main_entry_point(), ())
                .triangle_list()
                .viewports_dynamic_scissors_irrelevant(1)
                .fragment_shader(fs.main_entry_point(), ())
                .render_pass(Subpass::from(renderpass.clone(), 0).unwrap())
                .build(device.clone())
                .unwrap())
        };

        let fullscreen_vertex_buffer = CpuAccessibleBufferXalloc::<[VertexPosition]>::from_iter(
            device.clone(), BufferUsage::all(), vec![
                VertexPosition { position: [ -1.0,  1.0, 1.0 ] },
                VertexPosition { position: [  1.0,  1.0, 1.0 ] },
                VertexPosition { position: [  1.0, -1.0, 1.0 ] },
                VertexPosition { position: [ -1.0,  1.0, 1.0 ] },
                VertexPosition { position: [  1.0, -1.0, 1.0 ] },
                VertexPosition { position: [ -1.0, -1.0, 1.0 ] },
            ].iter().cloned()).expect("failed to create buffer");

        let descriptor_set = Arc::new(PersistentDescriptorSet::start(pipeline.clone(), 0)
            .add_image(scene_color).unwrap()
            .add_image(luma_out).unwrap()
            .build().unwrap());

        ResolveSceneColorStage {
            pipeline,
            framebuffers: None,
            framebuffer: None,
            renderpass,
            fullscreen_vertex_buffer,
            descriptor_set,
        }
    }
}

impl RenderStageDefinition for ResolveSceneColorStage {
    fn get_pipeline(&self) -> &Arc<dyn GraphicsPipelineAbstract + Send + Sync> { &self.pipeline }
    fn get_renderpass(&self) -> &Arc<dyn RenderPassAbstract + Send + Sync> { &self.renderpass }
    fn get_framebuffers(&self) -> &Option<Vec<Arc<dyn FramebufferAbstract + Send + Sync>>> { &self.framebuffers }
    fn get_framebuffers_mut(&mut self) -> &mut Option<Vec<Arc<dyn FramebufferAbstract + Send + Sync>>> { &mut self.framebuffers }

    fn build_command_buffers(&mut self, info: &RenderInfo) -> Option<Vec<(AutoCommandBuffer, Arc<Queue>)>> {
        let cb = AutoCommandBufferBuilder::primary_one_time_submit(info.device.clone(), info.queues.main.as_ref().unwrap().family())
            .unwrap()
            .begin_render_pass(self.framebuffer.as_ref().unwrap().clone(), false,
                               vec![ClearValue::None, ClearValue::None, [0.0, 0.0, 0.0, 1.0].into(), [0u32, 0, 0, 1].into()]).unwrap()
            .draw(self.get_pipeline().clone(), &DynamicState {
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
                                     vec![self.fullscreen_vertex_buffer.clone()],
                                     self.descriptor_set.clone(), ()).unwrap()
            .end_render_pass().unwrap();

        Some(vec![
            (cb.build().unwrap(), info.queues.main.as_ref().unwrap().clone()),
        ])
    }

    fn recreate_framebuffers_if_none(&mut self, images: &Vec<Arc<SwapchainImage<Window>>>, info: &RenderInfo) {
        // TODO: framebuffer sets for standalone mode
        if self.framebuffer.is_none() {
            self.framebuffer = Some(Arc::new(Framebuffer::start(self.get_renderpass().clone())
                // TODO: replace albedo hack with diffuse lighting
                .add(info.attachments.albedo.clone()).unwrap()
                .add(info.attachments.specular_light.clone()).unwrap()
                .add(info.attachments.scene_color.clone()).unwrap()
                .add(info.attachments.luma_render.clone()).unwrap()
                .build().unwrap()))
        }
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
