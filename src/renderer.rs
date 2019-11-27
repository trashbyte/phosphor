//! Main renderer.

use std::sync::{Arc, RwLock};

use cgmath::{EuclideanSpace, Matrix4, Vector4, SquareMatrix, Deg};
use winit::{Window, WindowBuilder, EventsLoop, MouseCursor};
use winit::dpi::LogicalSize;

use vulkano::buffer::BufferUsage;
use vulkano::device::{Device, DeviceExtensions, Queue};
use vulkano::format::{D32Sfloat, R16G16B16A16Sfloat, R32Uint, R32Sint};
use vulkano::image::attachment::AttachmentImage;
use vulkano::image::swapchain::SwapchainImage;
use vulkano::instance::{Instance, PhysicalDevice};
use vulkano::swapchain::{Swapchain, Surface, SwapchainCreationError, SwapchainAcquireFuture};
use vulkano::sync::GpuFuture;
use vulkano::image::ImageUsage;
use toolbox::Transform;

use crate::camera::Camera;
use crate::geometry::{VertexGroup, Material, VertexPositionObjectId, DeferredShadingVertex};
use crate::registry::TextureRegistry;
use crate::pipeline::{RenderPipelineAbstract, DeferredShadingRenderPipeline, DeferredLightingRenderPipeline, LinesRenderPipeline, TextRenderPipeline, OcclusionRenderPipeline, PostProcessRenderPipeline};
use crate::buffer::CpuAccessibleBufferXalloc;
use crate::geometry::VertexPositionColorAlpha;
use crate::pipeline::text::TextData;
use crate::pipeline::occlusion::OCCLUSION_FRAME_SIZE;
use crate::vulkano_win::VkSurfaceBuild;
use crate::pipeline::imgui::ImguiRenderPipeline;
use crate::compute::HistogramCompute;
use std::sync::atomic::Ordering;
use parking_lot::Mutex;


/// Matrix to correct vulkan clipping planes and flip y axis.
/// See [https://matthewwellings.com/blog/the-new-vulkan-coordinate-system/](https://matthewwellings.com/blog/the-new-vulkan-coordinate-system/).
pub static VULKAN_CORRECT_CLIP: Matrix4<f32> = Matrix4 {
    x: Vector4 { x: 1.0, y:  0.0, z: 0.0, w: 0.0 },
    y: Vector4 { x: 0.0, y: -1.0, z: 0.0, w: 0.0 },
    z: Vector4 { x: 0.0, y:  0.0, z: 0.5, w: 0.5 },
    w: Vector4 { x: 0.0, y:  0.0, z: 0.0, w: 1.0 }
};


pub const DEBUG_VISUALIZE_DISABLED: u32 = 0;
pub const DEBUG_VISUALIZE_POSITION_BUFFER: u32 = 1;
pub const DEBUG_VISUALIZE_NORMAL_BUFFER: u32 = 2;
pub const DEBUG_VISUALIZE_ALBEDO_BUFFER: u32 = 3;
pub const DEBUG_VISUALIZE_ROUGHNESS_BUFFER: u32 = 4;
pub const DEBUG_VISUALIZE_METALLIC_BUFFER: u32 = 5;
pub const DEBUG_VISUALIZE_DIFFUSE_LIGHTING_ONLY: u32 = 6;
pub const DEBUG_VISUALIZE_SPECULAR_LIGHTING_ONLY: u32 = 7;
pub const DEBUG_VISUALIZE_NO_POST_PROCESSING: u32 = 8;
pub const DEBUG_VISUALIZE_OCCLUSION_BUFFER: u32 = 9;
pub const DEBUG_VISUALIZE_MAX: u32 = 10;


lazy_static! {
    static ref GBUFFER_USAGE: ImageUsage = ImageUsage {
        color_attachment: true,
        input_attachment: true,
        ..ImageUsage::none()
    };
    static ref LUMA_BUFFER_USAGE: ImageUsage = ImageUsage {
        color_attachment: true,
        input_attachment: true,
        transfer_source: true,
        transfer_destination: true,
        ..ImageUsage::none()
    };
}


fn recreate_attachments(device: Arc<Device>, dimensions: [u32; 2], old_occlusion: Option<Arc<AttachmentImage<R32Uint>>>) -> RendererAttachments {
    RendererAttachments {
        position:     AttachmentImage::with_usage(device.clone(), dimensions, R16G16B16A16Sfloat, GBUFFER_USAGE.clone()).unwrap(),
        normal:       AttachmentImage::with_usage(device.clone(), dimensions, R16G16B16A16Sfloat, GBUFFER_USAGE.clone()).unwrap(),
        albedo:       AttachmentImage::with_usage(device.clone(), dimensions, R16G16B16A16Sfloat, GBUFFER_USAGE.clone()).unwrap(),
        roughness:    AttachmentImage::with_usage(device.clone(), dimensions, R16G16B16A16Sfloat, GBUFFER_USAGE.clone()).unwrap(),
        metallic:     AttachmentImage::with_usage(device.clone(), dimensions, R16G16B16A16Sfloat, GBUFFER_USAGE.clone()).unwrap(),
        hdr_diffuse:  AttachmentImage::with_usage(device.clone(), dimensions, R16G16B16A16Sfloat, GBUFFER_USAGE.clone()).unwrap(),
        hdr_specular: AttachmentImage::with_usage(device.clone(), dimensions, R16G16B16A16Sfloat, GBUFFER_USAGE.clone()).unwrap(),
        scene_color:  AttachmentImage::with_usage(device.clone(), dimensions, R16G16B16A16Sfloat, GBUFFER_USAGE.clone()).unwrap(),
        main_depth:   AttachmentImage::transient(device.clone(), dimensions, D32Sfloat).unwrap(),
        luma_render:  AttachmentImage::with_usage(device.clone(), dimensions, R32Sint, LUMA_BUFFER_USAGE.clone()).unwrap(),
        luma_mips:    AttachmentImage::with_usage(device.clone(), [512, 512], R32Sint, LUMA_BUFFER_USAGE.clone()).unwrap(),
        occlusion: old_occlusion
    }
}


#[derive(Debug)]
pub enum RendererDrawError {
    WindowMinimized,
    UnsupportedDimensions,
    SwapchainOutOfDate
}


#[derive(Clone)]
pub struct RendererAttachments {
    pub position: Arc<AttachmentImage<R16G16B16A16Sfloat>>,
    pub normal: Arc<AttachmentImage<R16G16B16A16Sfloat>>,
    pub albedo: Arc<AttachmentImage<R16G16B16A16Sfloat>>,
    pub roughness: Arc<AttachmentImage<R16G16B16A16Sfloat>>,
    pub metallic: Arc<AttachmentImage<R16G16B16A16Sfloat>>,
    pub hdr_diffuse: Arc<AttachmentImage<R16G16B16A16Sfloat>>,
    pub hdr_specular: Arc<AttachmentImage<R16G16B16A16Sfloat>>,
    pub scene_color: Arc<AttachmentImage<R16G16B16A16Sfloat>>,
    pub main_depth: Arc<AttachmentImage<D32Sfloat>>,
    pub luma_render: Arc<AttachmentImage<R32Sint>>,
    pub luma_mips: Arc<AttachmentImage<R32Sint>>,
    pub occlusion: Option<Arc<AttachmentImage<R32Uint>>>
}


#[derive(Clone)]
pub struct RenderInfo {
    /// Vulkan device.
    pub device: Arc<Device>,

    pub queue_main: Arc<Queue>,
    pub queue_offscreen: Arc<Queue>,
    pub queue_compute: Arc<Queue>,

    pub image_num: usize,
    pub dimensions: [u32; 2],
    pub camera_transform: Transform,
    pub view_mat: Matrix4<f32>,
    pub proj_mat: Matrix4<f32>,
    pub fov: Deg<f32>,
    pub tonemapping_info: TonemappingInfo,
    pub luma_avg_buffer: Arc<CpuAccessibleBufferXalloc<[u16]>>,
    pub histogram_compute: Arc<Mutex<HistogramCompute>>,

    pub tex_registry: Arc<TextureRegistry>,

    pub attachments: RendererAttachments,

    pub render_queues: Arc<RwLock<RenderQueues>>,

    pub debug_visualize_setting: u32,
}


pub enum GestaltRenderPass {
    Occlusion        = 0,
    DeferredShading  = 1,
    DeferredLighting = 2,
    PostProcess      = 3,
    Lines            = 4,
    Text             = 5,
    Imgui            = 6,
}


#[derive(Clone)]
pub struct TonemappingInfo {
    pub adjust_speed: f32,
    pub hist_low_percentile_bin: f32,
    pub hist_high_percentile_bin: f32,
    pub avg_scene_luma: f32,
    pub scene_ev100: f32,
    pub exposure: f32,
    pub exposure_adjustment: f32,
    pub min_exposure: f32,
    pub max_exposure: f32,
    pub vignette_opacity: f32
}
impl Default for TonemappingInfo {
    fn default() -> Self {
        Self {
            adjust_speed: 0.5,
            hist_low_percentile_bin: 0.0,
            hist_high_percentile_bin: 127.0,
            avg_scene_luma: 1.0,
            scene_ev100: 0.0,
            exposure: 0.5,
            min_exposure: 0.1,
            max_exposure: 3.0,
            exposure_adjustment: 0.0,
            vignette_opacity: 0.2
        }
    }
}


/// Queue of all objects to be drawn.
pub struct RenderQueues {
    pub occluders: OcclusionRenderQueue,
    pub meshes: Vec<MeshRenderQueueEntry>,
    pub lines: LineRenderQueue,
    pub text: Vec<TextData>,
}


/// Render queue entry for a single mesh
pub struct MeshRenderQueueEntry {
    pub vertex_group: Arc<VertexGroup<DeferredShadingVertex>>,
    pub material: Material,
    pub transform: Matrix4<f32>
}


/// Render queue for all lines to be drawn.
pub struct LineRenderQueue {
    pub chunk_lines_vg: Arc<VertexGroup<VertexPositionColorAlpha>>,
    pub chunks_changed: bool,
}

/// Render queue for the occlusion pass.
pub struct OcclusionRenderQueue {
    pub vertex_group: Arc<VertexGroup<VertexPositionObjectId>>,
    pub output_cpu_buffer: Arc<CpuAccessibleBufferXalloc<[u32]>>,
}


/// Main renderer.
pub struct Renderer {
    /// Vulkano surface.
    pub surface: Arc<Surface<Window>>,
    /// Vulkano swapchain.
    swapchain: Arc<Swapchain<Window>>,
    /// Swapchain images.
    images: Vec<Arc<SwapchainImage<Window>>>,
    /// If true, swapchain needs to be recreated.
    recreate_swapchain: bool,
    /// List of render pipelines.
    pipelines: Vec<Box<dyn RenderPipelineAbstract>>,
    /// Information required by render pipelines
    pub info: RenderInfo,
    imgui_pipeline: Option<ImguiRenderPipeline>
}


impl Renderer {
    /// Creates a new `Renderer`.
    pub fn new(event_loop: &EventsLoop) -> Renderer {
        let instance = Instance::new(None, &crate::vulkano_win::required_extensions(), None).expect("failed to create instance");
        let surface = WindowBuilder::new().with_dimensions(LogicalSize { width: 1366.0, height: 768.0 }).build_vk_surface(event_loop, instance.clone()).unwrap();
        let physical = PhysicalDevice::enumerate(&instance).next().expect("no device available");

        let device_ext = DeviceExtensions {
            khr_swapchain: true,
            khr_storage_buffer_storage_class: true,
            khr_dedicated_allocation: true,
            khr_16bit_storage: true,
            ..DeviceExtensions::none()
        };

        let family_graphics = physical.queue_families().find(|&q| q.supports_graphics() &&
            surface.is_supported(q).unwrap_or(false))
            .expect("couldn't find a graphical queue family (main)");
        let family_offscreen = physical.queue_families().find(|&q| q.supports_graphics())
            .expect("couldn't find a graphical queue family (offscreen)");
        let family_compute = physical.queue_families().find(|&q| q.supports_compute())
            .expect("couldn't find a compute queue family");

        let (device, mut queues) = Device::new(physical, physical.supported_features(),
                                               &device_ext,
                                               [(family_graphics, 0.9), (family_offscreen, 0.5), (family_compute, 0.4)].iter().cloned())
            .expect("failed to create device");
        let queue_main = queues.next().unwrap();
        let queue_offscreen = queues.next().unwrap();
        let queue_compute = queues.next().unwrap();
        let dimensions;
        let capabilities;
        let (swapchain, images) = {
            capabilities = surface.capabilities(physical.clone()).expect("failed to get surface capabilities");

            dimensions = [1366, 768];
            let usage = capabilities.supported_usage_flags;
            let alpha = capabilities.supported_composite_alpha.iter().next().unwrap();

            Swapchain::new(device.clone(), surface.clone(), capabilities.min_image_count,
                           vulkano::format::Format::B8G8R8A8Srgb, dimensions, 1, usage, &queue_main,
                           vulkano::swapchain::SurfaceTransform::Identity, alpha,
                           vulkano::swapchain::PresentMode::Immediate, true, None)
                .expect("failed to create swapchain")
        };

        let attachments = recreate_attachments(device.clone(), dimensions, None);

        let mut tex_registry = TextureRegistry::new();
        tex_registry.load(queue_main.clone());
        let tex_registry = Arc::new(tex_registry);

        let chunk_lines_vg = Arc::new(VertexGroup::new(Vec::<VertexPositionColorAlpha>::new().iter().cloned(), Vec::new().iter().cloned(), 0, device.clone()));
        let occlusion_vg = Arc::new(VertexGroup::new(Vec::<VertexPositionObjectId>::new().iter().cloned(), Vec::new().iter().cloned(), 0, device.clone()));
        let occlusion_cpu_buffer = CpuAccessibleBufferXalloc::<[u32]>::from_iter(device.clone(), BufferUsage::all(), vec![0u32; 320*240].iter().cloned()).expect("failed to create buffer");

        let luma_avg_buffer = CpuAccessibleBufferXalloc::from_iter(device.clone(), BufferUsage::transfer_destination(), [0u16; 4].iter().cloned()).unwrap();
        let histogram_compute = Arc::new(Mutex::new(HistogramCompute::new(device.clone())));

        let mut info = RenderInfo {
            device,
            image_num: 0,
            dimensions: [1024, 768],
            camera_transform: Transform::identity(),
            view_mat: Matrix4::identity(),
            proj_mat: Matrix4::identity(),
            fov: Deg(45f32),
            tonemapping_info: TonemappingInfo::default(),
            luma_avg_buffer,
            histogram_compute,
            tex_registry: tex_registry.clone(),
            queue_main,
            queue_offscreen,
            queue_compute,
            attachments,
            render_queues: Arc::new(RwLock::new(RenderQueues {
                lines: LineRenderQueue {
                    chunk_lines_vg,
                    chunks_changed: false,
                },
                text: Vec::new(),
                occluders: OcclusionRenderQueue {
                    vertex_group: occlusion_vg,
                    output_cpu_buffer: occlusion_cpu_buffer
                },
                meshes: Vec::new()
            })),
            debug_visualize_setting: DEBUG_VISUALIZE_DISABLED,
        };

        let mut pipelines = Vec::<Box<dyn RenderPipelineAbstract>>::with_capacity(3);
        pipelines.insert(GestaltRenderPass::Occlusion as usize,        Box::new(OcclusionRenderPipeline::new(&mut info, OCCLUSION_FRAME_SIZE)));
        pipelines.insert(GestaltRenderPass::DeferredShading as usize,  Box::new(DeferredShadingRenderPipeline::new(&info)));
        pipelines.insert(GestaltRenderPass::DeferredLighting as usize, Box::new(DeferredLightingRenderPipeline::new(&info)));
        pipelines.insert(GestaltRenderPass::PostProcess as usize,      Box::new(PostProcessRenderPipeline::new(&info)));
        pipelines.insert(GestaltRenderPass::Lines as usize,            Box::new(LinesRenderPipeline::new(&info)));
        pipelines.insert(GestaltRenderPass::Text as usize,             Box::new(TextRenderPipeline::new(&info)));

        Renderer {
            surface,
            swapchain,
            images,
            recreate_swapchain: false,
            pipelines,
            info,
            imgui_pipeline: None,
        }
    }

    pub fn with_imgui(mut self, imgui: &mut imgui::Context) -> Self {
        // TODO: dpi stuff
        imgui.io_mut().font_global_scale = 1.0;
        imgui.io_mut().display_framebuffer_scale = [1.0, 1.0];
        if let Some(logical_size) = self.surface.window().get_inner_size() {
            imgui.io_mut().display_size = [logical_size.width as f32, logical_size.height as f32];
        }
        self.imgui_pipeline = Some(ImguiRenderPipeline::new(&self.info, imgui));
        self
    }

    /// Draw all objects in the render queue. Called every frame in the game loop.
    pub fn draw(&mut self, camera: &Camera, _dt: f32, transform: Transform) -> Result<SwapchainAcquireFuture<Window>, RendererDrawError> {
        self.info.dimensions = match self.surface.window().get_inner_size() {
            Some(logical_size) => [logical_size.width as u32, logical_size.height as u32],
            None => [800, 600]
        };
        // minimizing window makes dimensions = [0, 0] which breaks swapchain creation.
        // skip draw loop until window is restored.
        if self.info.dimensions[0] < 1 || self.info.dimensions[1] < 1 {
            return Err(RendererDrawError::WindowMinimized);
        }

        self.info.view_mat = Matrix4::from(transform.rotation) * Matrix4::from_translation((transform.position * -1.0).to_vec());
        self.info.proj_mat = VULKAN_CORRECT_CLIP * cgmath::perspective(camera.fov, { self.info.dimensions[0] as f32 / self.info.dimensions[1] as f32 }, 0.1, 100.0);

        if self.recreate_swapchain {
            info!(Renderer, "Recreating swapchain");
            let (new_swapchain, new_images) = match self.swapchain.recreate_with_dimension(self.info.dimensions) {
                Ok(r) => r,
                Err(SwapchainCreationError::UnsupportedDimensions) => {
                    error!(Renderer, "SwapchainCreationError::UnsupportedDimensions");
                    return Err(RendererDrawError::UnsupportedDimensions);
                },
                Err(err) => panic!("{:?}", err)
            };

            std::mem::replace(&mut self.swapchain, new_swapchain);
            std::mem::replace(&mut self.images, new_images);

            self.info.attachments = recreate_attachments(self.info.device.clone(), self.info.dimensions,
                                                         Some(self.info.attachments.occlusion.as_ref().unwrap().clone()));

            for p in self.pipelines.iter_mut() {
                p.remove_framebuffers();
            }
            if let Some(p) = &mut self.imgui_pipeline {
                p.remove_framebuffers();
            }

            self.recreate_swapchain = false;
        }

        if !crate::compute::HISTOGRAM_COMPUTE_WORKING.load(Ordering::Relaxed) {
            self.info.histogram_compute.lock().submit(self.info.device.clone(), self.info.queue_compute.clone());
        }
        else {
            println!("histogram compute busy, skipping this frame");
        }

        for p in self.pipelines.iter_mut() {
            p.recreate_framebuffers_if_none(&self.images, &self.info);
        }
        if let Some(p) = &mut self.imgui_pipeline {
            p.recreate_framebuffers_if_none(&self.images, &self.info);
        }

        let (image_num, future) = match vulkano::swapchain::acquire_next_image(self.swapchain.clone(), None) {
            Ok(r) => r,
            Err(vulkano::swapchain::AcquireError::OutOfDate) => {
                self.recreate_swapchain = true;
                warn!(Renderer, "AcquireError::OutOfDate");
                return Err(RendererDrawError::SwapchainOutOfDate);
            },
            Err(err) => { fatal!(Renderer, "{:?}", err); }
        };
        self.info.image_num = image_num;

        self.info.fov = camera.fov.clone();
        self.info.camera_transform = transform.clone();
        let tonemap_info = self.info.tonemapping_info.clone();

        let low_bin;
        let high_bin;
        {
            let hist_lock = self.info.histogram_compute.lock();
            low_bin = hist_lock.low_percentile_bin;
            high_bin = hist_lock.high_percentile_bin;
        }

        let bin_avg = (low_bin + high_bin) / 2.0;
        let avg_log_luma = bin_avg / 4.6 - 10.0;
        let avg_luma = 2f32.powf(avg_log_luma);
        let ev100 = (avg_luma * 100.0 / 12.5).log2() + tonemap_info.exposure_adjustment;
        let max_luma = 1.2 * 2f32.powf(ev100);
        let exposure = 1.0 / max_luma;
        //let exposure = exposure.max(tonemap_info.min_exposure);

        self.info.tonemapping_info = TonemappingInfo {
            adjust_speed: 0.5,
            hist_low_percentile_bin: low_bin,
            hist_high_percentile_bin: high_bin,
            avg_scene_luma: avg_luma,
            scene_ev100: ev100,
            exposure,
            exposure_adjustment: tonemap_info.exposure_adjustment,
            min_exposure: tonemap_info.min_exposure,
            max_exposure: tonemap_info.max_exposure,
            vignette_opacity: tonemap_info.vignette_opacity
        };

        Ok(future)
    }

    pub fn draw_imgui(&mut self, ui: imgui::Ui) {
        match ui.mouse_cursor() {
            Some(mouse_cursor) => {
                self.surface.window().set_cursor(match mouse_cursor {
                    imgui::MouseCursor::Arrow => MouseCursor::Arrow,
                    imgui::MouseCursor::TextInput => MouseCursor::Text,
                    imgui::MouseCursor::ResizeAll => MouseCursor::Move,
                    imgui::MouseCursor::ResizeNS => MouseCursor::NsResize,
                    imgui::MouseCursor::ResizeEW => MouseCursor::EwResize,
                    imgui::MouseCursor::ResizeNESW => MouseCursor::NeswResize,
                    imgui::MouseCursor::ResizeNWSE => MouseCursor::NwseResize,
                    imgui::MouseCursor::Hand => MouseCursor::Hand,
                });
            }
            _ => self.surface.window().hide_cursor(true),
        }

        let draw_data = ui.render();
        self.imgui_pipeline.as_mut().unwrap().build_command_buffers(&self.info, draw_data);
    }

    pub fn submit(&mut self, image_acq_fut: SwapchainAcquireFuture<Window>) {
        let mut main_future:      Box<dyn GpuFuture> = Box::new(image_acq_fut);
        let occlusion_finished_future;

        let (cb, q) = self.pipelines[GestaltRenderPass::Occlusion as usize].build_command_buffer(&self.info);
        occlusion_finished_future = vulkano::sync::now(self.info.device.clone())
            .then_execute(q.clone(), cb).unwrap()
            .then_signal_semaphore_and_flush().unwrap();

        let (cb, q) = self.pipelines[GestaltRenderPass::DeferredShading as usize].build_command_buffer(&self.info);
        main_future = Box::new(main_future.then_execute(q.clone(), cb).unwrap());

        let (cb, q) = self.pipelines[GestaltRenderPass::DeferredLighting as usize].build_command_buffer(&self.info);
        main_future = Box::new(main_future.then_execute(q.clone(), cb).unwrap());

        let (cb, q) = self.pipelines[GestaltRenderPass::PostProcess as usize].build_command_buffer(&self.info);
        main_future = Box::new(main_future.join(occlusion_finished_future)
            .then_execute(q.clone(), cb).unwrap());

        let (cb, q) = self.pipelines[GestaltRenderPass::Lines as usize].build_command_buffer(&self.info);
        main_future = Box::new(main_future.then_execute(q.clone(), cb).unwrap());

        let (cb, q) = self.pipelines[GestaltRenderPass::Text as usize].build_command_buffer(&self.info);
        main_future = Box::new(main_future.then_execute(q.clone(), cb).unwrap());

        if self.imgui_pipeline.is_some() {
            if let Some(cbs) = self.imgui_pipeline.as_mut().unwrap().cached_command_buffers.take() {
                for cb in cbs {
                    main_future = Box::new(main_future.then_signal_semaphore().then_execute(self.info.queue_main.clone(), cb).unwrap());
                }
            }
        }

        let final_main_future = main_future.then_swapchain_present(self.info.queue_main.clone(),
                                                                  self.swapchain.clone(),
                                                                  self.info.image_num)
                                                                  .then_signal_fence_and_flush();
        match final_main_future {
            Ok(mut f) => {
                // This wait is required when using NVIDIA or running on macOS. See https://github.com/vulkano-rs/vulkano/issues/1247
                f.wait(None).unwrap();
                f.cleanup_finished();
            }
            Err(vulkano::sync::FlushError::OutOfDate) => {
                self.recreate_swapchain = true;
                return;
            }
            Err(e) => {
                error!(Renderer, "Error in submit(): {:?}", e);
            }
        }
    }
}
