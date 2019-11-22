use std::sync::Arc;
use imgui::{Textures, TextureId, DrawData, DrawCmd, DrawCmdParams};
use imgui::internal::RawWrapper;
use vulkano::device::Queue;
use vulkano::framebuffer::{Framebuffer, FramebufferAbstract, Subpass, RenderPassAbstract};
use vulkano::pipeline::{GraphicsPipeline, GraphicsPipelineAbstract};
use vulkano::image::{ImmutableImage, SwapchainImage};
use vulkano::sampler::{Sampler, Filter, MipmapMode, SamplerAddressMode};
use vulkano::buffer::BufferUsage;
use vulkano::command_buffer::{AutoCommandBufferBuilder, DynamicState, AutoCommandBuffer};
use vulkano::pipeline::viewport::{Scissor, Viewport};
use vulkano::descriptor::descriptor_set::PersistentDescriptorSet;
use vulkano::format::R8G8B8A8Srgb;

use crate::renderer::RenderInfo;
use crate::pipeline::RenderPipelineAbstract;
use crate::buffer::CpuAccessibleBufferXalloc;
use winit::Window;
use vulkano::sync::GpuFuture;


mod shaders {
    pub mod vertex {
        vulkano_shaders::shader!{
            ty: "vertex",
            path: "src/pipeline/imgui/imgui.vert"
        }
    }
    pub mod fragment {
        vulkano_shaders::shader!{
            ty: "fragment",
            path: "src/pipeline/imgui/imgui.frag"
        }
    }
}


#[derive(Debug, Clone, Default)]
struct Vertex {
    pub pos: [f32; 2],
    pub uv: [f32; 2],
    pub col: [f32; 4],
}
impl_vertex!(Vertex, pos, uv, col);


pub struct ImguiRenderPipeline {
    queue: Arc<Queue>,
    pipeline: Arc<dyn GraphicsPipelineAbstract + Send + Sync>,
    pub framebuffers: Option<Vec<Arc<dyn FramebufferAbstract + Send + Sync>>>,
    renderpass: Arc<dyn RenderPassAbstract + Send + Sync>,
    font_texture: Arc<ImmutableImage<R8G8B8A8Srgb>>,
    textures: Textures<Arc<ImmutableImage<R8G8B8A8Srgb>>>,
    sampler: Arc<Sampler>,
    pub cached_command_buffers: Option<Vec<AutoCommandBuffer>>,
}


impl ImguiRenderPipeline {
    pub fn new(info: &RenderInfo, ctx: &mut imgui::Context) -> Self {
        let vs = shaders::vertex::Shader::load(info.device.clone()).expect("failed to create shader module");
        let fs = shaders::fragment::Shader::load(info.device.clone()).expect("failed to create shader module");

        let renderpass = Arc::new(vulkano::single_pass_renderpass!(
            info.device.clone(),
            attachments: {
                color: {
                    load: Load,
                    store: Store,
                    format: vulkano::format::Format::B8G8R8A8Srgb,
                    samples: 1,
                }
            },
            pass: {
                color: [color],
                depth_stencil: {}
            }
        ).unwrap());

        let (font_texture, _future) = {
            let mut fonts = ctx.fonts();
            let texture = fonts.build_rgba32_texture();
            vulkano::image::immutable::ImmutableImage::from_iter(
                texture.data.iter().cloned(),
                vulkano::image::Dimensions::Dim2d { width: texture.width, height: texture.height },
                R8G8B8A8Srgb,
                info.queue_main.clone()).unwrap()
        };
        _future.then_signal_fence_and_flush().unwrap().wait(None).unwrap();

        let pipeline = Arc::new(GraphicsPipeline::start()
            .cull_mode_disabled()
            .vertex_input_single_buffer::<Vertex>()
            .vertex_shader(vs.main_entry_point(), ())
            .triangle_list()
            .viewports_scissors_dynamic(1)
            .fragment_shader(fs.main_entry_point(), ())
            .blend_alpha_blending()
            .render_pass(Subpass::from(renderpass.clone(), 0).unwrap())
            .build(info.device.clone())
            .unwrap());

        let sampler = Sampler::new(info.device.clone(), Filter::Linear, Filter::Linear, MipmapMode::Linear,
                                   SamplerAddressMode::Repeat, SamplerAddressMode::Repeat, SamplerAddressMode::Repeat,
                                   0.0, 4.0, 0.0, 0.0).unwrap();

        ImguiRenderPipeline {
            queue: info.queue_main.clone(),
            pipeline,
            framebuffers: None,
            renderpass,
            font_texture,
            textures: Textures::new(),
            sampler,
            cached_command_buffers: None
        }
    }

    pub fn reload_font_texture(&mut self, ctx: &mut imgui::Context) {
        let (font_texture, _future) = {
            let mut fonts = ctx.fonts();
            let texture = fonts.build_rgba32_texture();
            vulkano::image::immutable::ImmutableImage::from_iter(
                texture.data.iter().cloned(),
                vulkano::image::Dimensions::Dim2d { width: texture.width, height: texture.height },
                R8G8B8A8Srgb,
                self.queue.clone()).unwrap()
        };
        // TODO: probably unnecessary (one above in new() too)
        _future.then_signal_fence_and_flush().unwrap().wait(None).unwrap();
        self.font_texture = font_texture;
    }
    pub fn textures(&mut self) -> &mut Textures<Arc<ImmutableImage<R8G8B8A8Srgb>>> {
        &mut self.textures
    }
    fn lookup_texture(&self, texture_id: TextureId) -> Result<Arc<ImmutableImage<R8G8B8A8Srgb>>, String> {
        if texture_id.id() == 0 {
            Ok(self.font_texture.clone())
        } else if let Some(texture) = self.textures.get(texture_id) {
            Ok(texture.clone())
        } else {
            Err(format!("Bad Texture id: {:?}", texture_id))
        }
    }
    pub fn build_command_buffers(&mut self, info: &RenderInfo, draw_data: &DrawData) {
        let fb_width = draw_data.display_size[0] * draw_data.framebuffer_scale[0];
        let fb_height = draw_data.display_size[1] * draw_data.framebuffer_scale[1];
        if !(fb_width > 0.0 && fb_height > 0.0) {
            return;
        }
        let left = draw_data.display_pos[0];
        let right = draw_data.display_pos[0] + draw_data.display_size[0];
        let bottom = draw_data.display_pos[1];
        let top = draw_data.display_pos[1] + draw_data.display_size[1];
        let matrix = [
            [(2.0 / (right - left)),      0.0,                         0.0, 0.0],
            [0.0,                         (2.0/(top-bottom)),          0.0, 0.0],
            [0.0,                         0.0,                        -1.0, 0.0],
            [(right+left) / (left-right), (top+bottom) / (bottom-top), 0.0, 1.0 ],
        ];
        let clip_off = draw_data.display_pos;
        let clip_scale = draw_data.framebuffer_scale;
        let mut cbs = Vec::new();
        for draw_list in draw_data.draw_lists() {
            let vtx_buffer = CpuAccessibleBufferXalloc::from_iter(
                info.device.clone(), BufferUsage::vertex_buffer(),
                draw_list.vtx_buffer()
                    .iter()
                    .map(|v| { Vertex { pos: v.pos, uv: v.uv, col: [v.col[0] as f32, v.col[1] as f32, v.col[2] as f32, v.col[3] as f32] } })).unwrap();
            let mut idx_start = 0;
            let mut cb = AutoCommandBufferBuilder::primary_one_time_submit(info.device.clone(), info.queue_main.family()).unwrap()
                .begin_render_pass(self.framebuffers.as_ref().unwrap()[info.image_num].clone(), false, vec![vulkano::format::ClearValue::None]).unwrap();
            for cmd in draw_list.commands() {
                match cmd {
                    DrawCmd::Elements {
                        count, cmd_params: DrawCmdParams { clip_rect, texture_id, .. },
                    } => {
                        let idx_end = idx_start + count;
                        // TODO: don't make new buffers for every draw
                        let idx_buffer = CpuAccessibleBufferXalloc::from_iter(info.device.clone(), BufferUsage::index_buffer(), draw_list.idx_buffer().iter().skip(idx_start).take(count).map(|i| { *i })).unwrap();
                        let clip_rect = [
                            (clip_rect[0] - clip_off[0]) * clip_scale[0],
                            (clip_rect[1] - clip_off[1]) * clip_scale[1],
                            (clip_rect[2] - clip_off[0]) * clip_scale[0],
                            (clip_rect[3] - clip_off[1]) * clip_scale[1],
                        ];

                        if clip_rect[0] < fb_width && clip_rect[1] < fb_height && clip_rect[2] >= 0.0 && clip_rect[3] >= 0.0 {
                            let set;
                            match self.lookup_texture(texture_id) {
                                Ok(t) => {
                                    set = PersistentDescriptorSet::start(self.pipeline.clone(), 0)
                                        .add_sampled_image(t.clone(), self.sampler.clone()).unwrap()
                                        .build().unwrap();
                                },
                                Err(e) => {
                                    println!("{:?}", e);
                                    set = PersistentDescriptorSet::start(self.pipeline.clone(), 0)
                                        .add_sampled_image(info.tex_registry.get("white").unwrap().clone(), self.sampler.clone()).unwrap()
                                        .build().unwrap();
                                }
                            }

                            cb = cb.draw_indexed(self.pipeline.clone(), &DynamicState {
                                line_width: None,
                                viewports: Some(vec![Viewport {
                                    origin: [0.0, 0.0],
                                    dimensions: [fb_width as f32, fb_height as f32],
                                    depth_range: 0.0..1.0,
                                }]),
                                scissors: Some(vec![Scissor {
                                    origin: [f32::max(0.0, clip_rect[0]).floor() as i32,
                                        f32::max(0.0, clip_rect[1]).floor() as i32],
                                    dimensions: [(clip_rect[2] - clip_rect[0]).abs().ceil() as u32,
                                                 (clip_rect[3] - clip_rect[1]).abs().ceil() as u32]
                                }]),
                                compare_mask: None,
                                write_mask: None,
                                reference: None
                            },
                                                 vec![vtx_buffer.clone()],
                                                 idx_buffer,
                                                 set, shaders::vertex::ty::Constants {
                                    matrix
                                }).unwrap();
                        }
                        idx_start = idx_end;
                    }
                    DrawCmd::ResetRenderState => (), // TODO
                    DrawCmd::RawCallback { callback, raw_cmd } => unsafe {
                        callback(draw_list.raw(), raw_cmd)
                    },
                }
            }
            let cb = cb.end_render_pass().unwrap().build().unwrap();
            cbs.push(cb);
        }

        self.cached_command_buffers = Some(cbs);
    }
}

impl RenderPipelineAbstract for ImguiRenderPipeline {
    fn get_framebuffers_mut(&mut self) -> &mut Option<Vec<Arc<dyn FramebufferAbstract + Send + Sync>>> {
        &mut self.framebuffers
    }

    fn get_renderpass(&self) -> Arc<dyn RenderPassAbstract + Send + Sync> {
        self.renderpass.clone() as Arc<dyn RenderPassAbstract + Send + Sync>
    }

    fn build_command_buffer(&mut self, _info: &RenderInfo) -> (AutoCommandBuffer, Arc<Queue>) {
        unimplemented!();
    }

    fn recreate_framebuffers_if_none(&mut self, images: &Vec<Arc<SwapchainImage<Window>>>, _info: &RenderInfo) {
        if self.get_framebuffers_mut().is_none() {
            let new_framebuffers = Some(images.iter().map(|image| {
                let arc: Arc<dyn FramebufferAbstract + Send + Sync> = Arc::new(Framebuffer::start(self.get_renderpass().clone())
                    .add(image.clone()).unwrap()
                    .build().unwrap());
                arc
            }).collect::<Vec<_>>());
            ::std::mem::replace(self.get_framebuffers_mut(), new_framebuffers);
        }
    }
}
