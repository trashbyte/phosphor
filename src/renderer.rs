//! Main renderer.

use std::sync::Arc;

use cgmath::{Matrix4, Vector4, SquareMatrix, Deg};
use winit::{Window, WindowBuilder, EventsLoop};
use winit::dpi::LogicalSize;

use vulkano::device::{Device, DeviceExtensions, Queue};
use vulkano::format::{D32Sfloat, R16G16B16A16Sfloat, R32Uint, B8G8R8A8Srgb};
use vulkano::image::attachment::AttachmentImage;
use vulkano::image::swapchain::SwapchainImage;
use vulkano::instance::{Instance, PhysicalDevice};
use vulkano::swapchain::{Swapchain, Surface};
use vulkano::sync::GpuFuture;
use vulkano::image::ImageUsage;
use vulkano::command_buffer::{AutoCommandBufferBuilder, DynamicState};

use toolbelt::Transform;

use crate::geometry::{Mesh, MeshVertex, VertexPosition};
use crate::vulkano_win::VkSurfaceBuild;
use crate::material::{MaterialDefinition, SkyboxMaterial};
use hashbrown::HashMap;
use crate::stage::mesh_shading::GenericMeshShadingStage;
use crate::stage::RenderStageDefinition;
use parking_lot::Mutex;
use crate::material::params::MaterialParams;
use vulkano::sampler::Filter;
use crate::stage::resolve_scene_color::ResolveSceneColorStage;
use vulkano::pipeline::viewport::Viewport;
use vulkano::pipeline::GraphicsPipeline;
use vulkano::framebuffer::{Subpass, Framebuffer};
use crate::buffer::CpuAccessibleBufferXalloc;
use vulkano::buffer::BufferUsage;

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

pub const OCCLUSION_FRAME_SIZE: [u32; 2] = [256, 144];

#[derive(Debug)]
pub enum RendererDrawError {
    WindowMinimized,
    UnsupportedDimensions,
    SwapchainOutOfDate
}

lazy_static! {
    static ref GBUFFER_USAGE: ImageUsage = ImageUsage {
        color_attachment: true,
        input_attachment: true,
        transfer_source: true, // TODO: remove me when there's proper output
        ..ImageUsage::none()
    };
    static ref LUMA_BUFFER_USAGE: ImageUsage = ImageUsage {
        color_attachment: true,
        input_attachment: true,
        transfer_source: true,
        transfer_destination: true,
        ..ImageUsage::none()
    };
    static ref OCCLUSION_BUFFER_USAGE: ImageUsage = ImageUsage {
         color_attachment: true,
         transfer_source: true,
         sampled: true,
         ..ImageUsage::none()
     };
}

pub struct Attachments {
    pub position: Arc<AttachmentImage<R16G16B16A16Sfloat>>,
    pub normal: Arc<AttachmentImage<R16G16B16A16Sfloat>>,
    pub albedo: Arc<AttachmentImage<R16G16B16A16Sfloat>>,
    pub roughness: Arc<AttachmentImage<R16G16B16A16Sfloat>>,
    pub metallic: Arc<AttachmentImage<R16G16B16A16Sfloat>>,
    pub diffuse_light: Arc<AttachmentImage<R16G16B16A16Sfloat>>,
    pub specular_light: Arc<AttachmentImage<R16G16B16A16Sfloat>>,
    pub scene_color: Arc<AttachmentImage<R16G16B16A16Sfloat>>,
    pub main_depth: Arc<AttachmentImage<D32Sfloat>>,
    pub luma_render: Arc<AttachmentImage<R32Uint>>,
}

fn recreate_attachments(device: Arc<Device>, dimensions: [u32; 2]) -> Attachments {
    Attachments {
        position: AttachmentImage::with_usage(device.clone(), dimensions, R16G16B16A16Sfloat, *GBUFFER_USAGE).unwrap(),
        normal: AttachmentImage::with_usage(device.clone(), dimensions, R16G16B16A16Sfloat, *GBUFFER_USAGE).unwrap(),
        albedo: AttachmentImage::with_usage(device.clone(), dimensions, R16G16B16A16Sfloat, *GBUFFER_USAGE).unwrap(),
        roughness: AttachmentImage::with_usage(device.clone(), dimensions, R16G16B16A16Sfloat, *GBUFFER_USAGE).unwrap(),
        metallic: AttachmentImage::with_usage(device.clone(), dimensions, R16G16B16A16Sfloat, *GBUFFER_USAGE).unwrap(),
        diffuse_light: AttachmentImage::with_usage(device.clone(), dimensions, R16G16B16A16Sfloat, *GBUFFER_USAGE).unwrap(),
        specular_light: AttachmentImage::with_usage(device.clone(), dimensions, R16G16B16A16Sfloat, *GBUFFER_USAGE).unwrap(),
        scene_color: AttachmentImage::with_usage(device.clone(), dimensions, R16G16B16A16Sfloat, *GBUFFER_USAGE).unwrap(),
        main_depth: AttachmentImage::transient(device.clone(), dimensions, D32Sfloat).unwrap(),
        luma_render: AttachmentImage::with_usage(device.clone(), dimensions, R32Uint, *LUMA_BUFFER_USAGE).unwrap(),
    }
}

pub struct RenderInfo {
    pub device: Arc<Device>,
    pub queues: Queues,
    pub dimensions: [u32; 2],
    pub camera_transform: Transform,
    pub view_mat: Matrix4<f32>,
    pub proj_mat: Matrix4<f32>,
    pub fov: Deg<f32>,
    pub tonemapping_info: TonemappingInfo,
    pub debug_visualize_setting: u32,
    pub image_num: usize,
    pub mesh_queue: Mutex<Vec<Mesh>>,
    pub materials: HashMap<String, Arc<dyn MaterialDefinition + Send + Sync>>,
    pub attachments: Attachments,
}
impl RenderInfo {
    fn new(device: Arc<Device>, queues: Queues, dimensions: [u32; 2]) -> Self {
        Self {
            device: device.clone(),
            queues,
            dimensions,
            camera_transform: Transform::identity(),
            view_mat: Matrix4::identity(),
            proj_mat: cgmath::perspective(Deg(45f32), dimensions[0] as f32 / dimensions[1] as f32, 0.1, 10000.0),
            fov: Deg(45f32),
            tonemapping_info: TonemappingInfo::default(),
            debug_visualize_setting: DEBUG_VISUALIZE_DISABLED,
            image_num: 0,
            mesh_queue: Mutex::new(Vec::new()),
            materials: HashMap::new(),
            attachments: recreate_attachments(device.clone(), dimensions),
        }
    }
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

#[derive(Debug, Clone)]
pub struct Queues {
    pub main: Option<Arc<Queue>>,
    pub offscreen: Option<Arc<Queue>>,
    pub compute: Option<Arc<Queue>>,
}
impl Queues {
    pub fn none() -> Self { Self { main: None, offscreen: None, compute: None } }
}


pub struct PhosphorRendererBuilder<'a> {
    event_loop: Option<&'a EventsLoop>,
    dimensions: Option<(f64, f64)>,
    extensions: Option<DeviceExtensions>,
    embedded_info: Option<EmbeddedModeInfo>,
    device: Option<Arc<Device>>,
    queues: Queues,
}


impl<'a> PhosphorRendererBuilder<'a> {
    pub(crate) fn new_standalone(event_loop: &'a EventsLoop) -> Self {
        Self {
            event_loop: Some(event_loop),
            dimensions: None,
            extensions: None,
            embedded_info: None,
            device: None,
            queues: Queues::none(),
        }
    }

    pub(crate) fn new_embedded(queues: Queues, render_target: Arc<AttachmentImage<B8G8R8A8Srgb>>) -> Self {
        let device;
        if let Some(q) = queues.main.as_ref() {
            device = Some(q.device().clone());
        }
        else if let Some(q) = queues.offscreen.as_ref() {
            device = Some(q.device().clone());
        }
        else if let Some(q) = queues.compute.as_ref() {
            device = Some(q.device().clone());
        }
        else {
            fatal!(Renderer, "Cannot initialize renderer without any queues!");
        }

        Self {
            event_loop: None,
            dimensions: None,
            extensions: None,
            embedded_info: Some(EmbeddedModeInfo { render_target }),
            device,
            queues,
        }
    }

    pub fn with_dimensions(mut self, width: f64, height: f64) -> Self {
        self.dimensions = Some((width, height));
        self
    }

    pub fn with_extensions(mut self, extensions: DeviceExtensions) -> Self {
        self.extensions = Some(extensions);
        self
    }

    pub fn build(self) -> PhosphorRenderer {
        let dimensions = match self.dimensions {
            Some((width, height)) => [width as u32, height as u32],
            None => [1366, 768],
        };
        let logical_dimensions = LogicalSize { width: dimensions[0] as f64, height: dimensions[1] as f64 };

        match self.embedded_info {
            Some(embedded_info) => {
                let device = self.device.unwrap().clone();
                let queues = self.queues.clone();

                let mut info = RenderInfo::new(device.clone(), queues.clone(), dimensions);

                let stages = RendererStages::new(&info);

                info.materials.insert("skybox".to_string(), Arc::new(
                    SkyboxMaterial::new(&info, stages.mesh_shading.get_renderpass().clone(), 0, MaterialParams::new()))
                );

                PhosphorRenderer {
                    mode: RendererMode::Embedded(embedded_info),
                    device: device.clone(),
                    queues: queues.clone(),
                    info,
                    params: Default::default(),
                    stages,
                }
            },
            None => {
                let instance = Instance::new(None, &crate::vulkano_win::required_extensions(), None).expect("failed to create instance");
                let surface = WindowBuilder::new().with_dimensions(logical_dimensions)
                    .build_vk_surface(self.event_loop.unwrap(), instance.clone())
                    .unwrap();

                let physical = PhysicalDevice::enumerate(&instance).next().expect("no device available");

                let full_ext = match self.extensions {
                    Some(ext) => DeviceExtensions { khr_swapchain: true, ..ext },
                    None => DeviceExtensions { khr_swapchain: true, ..DeviceExtensions::none() }
                };

                // TODO: support mixed queues in standalone mode
                let family_graphics = physical.queue_families().find(|&q| q.supports_graphics() &&
                    surface.is_supported(q).unwrap_or(false))
                    .expect("couldn't find a graphical queue family (main)");
                let family_offscreen = physical.queue_families().find(|&q| q.supports_graphics())
                    .expect("couldn't find a graphical queue family (offscreen)");
                let family_compute = physical.queue_families().find(|&q| q.supports_compute())
                    .expect("couldn't find a compute queue family");

                let (device, mut queues) = Device::new(physical, physical.supported_features(),
                                                       &full_ext,
                                                       [(family_graphics, 0.9), (family_offscreen, 0.5), (family_compute, 0.4)].iter().cloned())
                    .expect("failed to create device");
                let queues = Queues {
                    main: Some(queues.next().unwrap()),
                    offscreen: Some(queues.next().unwrap()),
                    compute: Some(queues.next().unwrap()),
                };

                let capabilities;
                let (swapchain, images) = {
                    capabilities = surface.capabilities(physical.clone()).expect("failed to get surface capabilities");

                    let usage = capabilities.supported_usage_flags;
                    let alpha = capabilities.supported_composite_alpha.iter().next().unwrap();

                    Swapchain::new(device.clone(), surface.clone(), capabilities.min_image_count,
                                   vulkano::format::Format::B8G8R8A8Srgb, dimensions, 1, usage,
                                   queues.main.as_ref().expect("main queue is currently required in standalone mode"),
                                   vulkano::swapchain::SurfaceTransform::Identity, alpha,
                                   vulkano::swapchain::PresentMode::Fifo, true, None)
                        .expect("failed to create swapchain")
                };

                let mut info = RenderInfo::new(device.clone(), queues.clone(), dimensions);

                let stages = RendererStages::new(&info);

                info.materials.insert("skybox".to_string(), Arc::new(
                    SkyboxMaterial::new(&info, stages.mesh_shading.get_renderpass().clone(), 0, MaterialParams::new()))
                );

                PhosphorRenderer {
                    mode: RendererMode::Standalone(StandaloneModeInfo {
                        surface,
                        swapchain,
                        images,
                        image_num: 0,
                        recreate_swapchain: false,
                    }),
                    device: device.clone(),
                    queues: queues.clone(),
                    info,
                    params: Default::default(),
                    stages,
                }
            }
        }
    }
}


pub struct StandaloneModeInfo {
    /// Vulkano surface.
    pub surface: Arc<Surface<Window>>,
    /// Vulkano swapchain.
    pub swapchain: Arc<Swapchain<Window>>,
    /// Swapchain images.
    pub images: Vec<Arc<SwapchainImage<Window>>>,
    /// Index of currently acquired swapchain image
    pub image_num: usize,
    /// If true, swapchain needs to be recreated.
    pub recreate_swapchain: bool,
}
pub struct EmbeddedModeInfo {
    render_target: Arc<AttachmentImage<B8G8R8A8Srgb>>
}
pub enum RendererMode {
    Standalone(StandaloneModeInfo),
    Embedded(EmbeddedModeInfo)
}

/// Struct for info passed into the renderer from the crate using phosphor
pub struct RendererParams {
    pub camera_transform: Transform
}
impl Default for RendererParams {
    fn default() -> Self {
        Self {
            camera_transform: Transform::identity()
        }
    }
}

pub struct RendererStages {
    mesh_shading: GenericMeshShadingStage,
    resolve_scene_color: ResolveSceneColorStage,
}
impl RendererStages {
    pub fn new(info: &RenderInfo) -> Self {
        Self {
            mesh_shading: GenericMeshShadingStage::new(info.device.clone()),
            resolve_scene_color: ResolveSceneColorStage::new(info.device.clone(),
                                                             info.attachments.scene_color.clone(),
                                                             info.attachments.luma_render.clone()),
        }
    }
    pub fn recreate_framebuffers_if_none(&mut self, images: &Vec<Arc<SwapchainImage<Window>>>, info: &RenderInfo) {
        self.mesh_shading.recreate_framebuffers_if_none(images, info);
        self.resolve_scene_color.recreate_framebuffers_if_none(images, info);
    }
}

/// Main renderer.
pub struct PhosphorRenderer {
    mode: RendererMode,
    device: Arc<Device>,
    queues: Queues,
    pub info: RenderInfo,
    params: RendererParams,
    stages: RendererStages,
}


impl PhosphorRenderer {
    pub fn create_standalone(event_loop: &EventsLoop) -> PhosphorRendererBuilder { PhosphorRendererBuilder::new_standalone(event_loop) }

    pub fn create_embedded(queues: Queues, render_target: Arc<AttachmentImage<B8G8R8A8Srgb>>) -> PhosphorRendererBuilder<'static> { PhosphorRendererBuilder::new_embedded(queues, render_target) }

    pub fn update(&mut self, update: RendererParams) {
        self.params = update;
    }

    pub fn queue_mesh(&mut self, mesh: Mesh) {
        self.info.mesh_queue.lock().push(mesh);
    }

//        // minimizing window makes dimensions = [0, 0] which breaks swapchain creation.
//        // skip draw loop until window is restored.
//        if self.info.dimensions[0] < 1 || self.info.dimensions[1] < 1 {
//            return Err(RendererDrawError::WindowMinimized);
//        }
//
//        self.info.view_mat = Matrix4::from(transform.rotation) * Matrix4::from_translation((transform.position * -1.0).to_vec());
//        self.info.proj_mat = VULKAN_CORRECT_CLIP * cgmath::perspective(camera.fov, { self.info.dimensions[0] as f32 / self.info.dimensions[1] as f32 }, 0.1, 100.0);
//
//        if self.recreate_swapchain {
//            info!(Renderer, "Recreating swapchain");
//            let (new_swapchain, new_images) = match self.swapchain.recreate_with_dimension(self.info.dimensions) {
//                Ok(r) => r,
//                Err(SwapchainCreationError::UnsupportedDimensions) => {
//                    error!(Renderer, "SwapchainCreationError::UnsupportedDimensions");
//                    return Err(RendererDrawError::UnsupportedDimensions);
//                },
//                Err(err) => panic!("{:?}", err)
//            };
//
//            std::mem::replace(&mut self.swapchain, new_swapchain);
//            std::mem::replace(&mut self.images, new_images);
//
//            self.info.attachments = recreate_attachments(self.info.device.clone(), self.info.dimensions,
//                                                         Some(self.info.attachments.occlusion.as_ref().unwrap().clone()));
//
//            for p in self.pipelines.iter_mut() {
//                p.remove_framebuffers();
//            }
//            if let Some(p) = &mut self.imgui_pipeline {
//                p.remove_framebuffers();
//            }
//
//            self.recreate_swapchain = false;
//        }
//
//        if !crate::compute::HISTOGRAM_COMPUTE_WORKING.load(Ordering::Relaxed) {
//            self.info.histogram_compute.lock().submit(self.info.device.clone(), self.info.queue_compute.clone());
//        }
//        else {
//            println!("histogram compute busy, skipping this frame");
//        }
//
//        for p in self.pipelines.iter_mut() {
//            p.recreate_framebuffers_if_none(&self.images, &self.info);
//        }
//        if let Some(p) = &mut self.imgui_pipeline {
//            p.recreate_framebuffers_if_none(&self.images, &self.info);
//        }
//
//        let (image_num, future) = match vulkano::swapchain::acquire_next_image(self.swapchain.clone(), None) {
//            Ok(r) => r,
//            Err(vulkano::swapchain::AcquireError::OutOfDate) => {
//                self.recreate_swapchain = true;
//                warn!(Renderer, "AcquireError::OutOfDate");
//                return Err(RendererDrawError::SwapchainOutOfDate);
//            },
//            Err(err) => { fatal!(Renderer, "{:?}", err); }
//        };
//        self.info.image_num = image_num;
//
//        self.info.fov = camera.fov.clone();
//        self.info.camera_transform = transform.clone();
//        let tonemap_info = self.info.tonemapping_info.clone();
//
//        let low_bin;
//        let high_bin;
//        {
//            let hist_lock = self.info.histogram_compute.lock();
//            low_bin = hist_lock.low_percentile_bin;
//            high_bin = hist_lock.high_percentile_bin;
//        }
//
//        let bin_avg = (low_bin + high_bin) / 2.0;
//        let avg_log_luma = bin_avg / 4.6 - 10.0;
//        let avg_luma = 2f32.powf(avg_log_luma);
//        let ev100 = (avg_luma * 100.0 / 12.5).log2() + tonemap_info.exposure_adjustment;
//        let max_luma = 1.2 * 2f32.powf(ev100);
//        let exposure = 1.0 / max_luma;
//        //let exposure = exposure.max(tonemap_info.min_exposure);
//
//        self.info.tonemapping_info = TonemappingInfo {
//            adjust_speed: 0.5,
//            hist_low_percentile_bin: low_bin,
//            hist_high_percentile_bin: high_bin,
//            avg_scene_luma: avg_luma,
//            scene_ev100: ev100,
//            exposure,
//            exposure_adjustment: tonemap_info.exposure_adjustment,
//            min_exposure: tonemap_info.min_exposure,
//            max_exposure: tonemap_info.max_exposure,
//            vignette_opacity: tonemap_info.vignette_opacity
//        };
//
//        Ok(future)

    pub fn submit(&mut self, skybox: &Mesh) -> Box<dyn GpuFuture> {
        self.stages.recreate_framebuffers_if_none(&mut vec![], &self.info);

        match &self.mode {
            RendererMode::Standalone(_) => unimplemented!(),
            RendererMode::Embedded(info) => {
//                let mut command_buffers = Vec::new();
//
//                if let Some(cbs) = self.stages.mesh_shading.build_command_buffers(&self.info) {
//                    command_buffers.extend(cbs.into_iter());
//                }
//                if let Some(cbs) = self.stages.resolve_scene_color.build_command_buffers(&self.info) {
//                    command_buffers.extend(cbs.into_iter());
//                }

//                let mut future: Box<dyn GpuFuture> = Box::new(vulkano::sync::now(self.device.clone()));
//
//                for (cb, q) in command_buffers {
//                    future = Box::new(future.then_execute(q.clone(), cb).unwrap());
//                }
//
//                let output_cb = AutoCommandBufferBuilder::primary_one_time_submit(self.info.device.clone(), self.info.queues.main.as_ref().unwrap().family()).unwrap()
//                    .blit_image(self.info.attachments.albedo.clone(), [0, 0, 0], [self.info.dimensions[0] as i32, self.info.dimensions[1] as i32, 1], 0, 0,
//                                info.render_target.clone(), [0, 0, 0], [self.info.dimensions[0] as i32, self.info.dimensions[1] as i32, 1], 0, 0, 1, Filter::Linear).unwrap()
//                    .build().unwrap();
                let embedded_info = match &self.mode {
                    RendererMode::Embedded(e) => e,
                    RendererMode::Standalone(_) => panic!()
                };

                let fb = Arc::new(Framebuffer::start(self.stages.mesh_shading.get_renderpass().clone())
                    .add(embedded_info.render_target.clone()).unwrap()
                    .build().unwrap());
                let fullscreen_vertex_buffer = CpuAccessibleBufferXalloc::<[VertexPosition]>::from_iter(
                    self.device.clone(), BufferUsage::all(), vec![
                        VertexPosition { position: [ -1.0,  1.0, 0.5 ] },
                        VertexPosition { position: [  1.0,  1.0, 0.5 ] },
                        VertexPosition { position: [  1.0, -1.0, 0.5 ] },
                        VertexPosition { position: [ -1.0,  1.0, 0.5 ] },
                        VertexPosition { position: [  1.0, -1.0, 0.5 ] },
                        VertexPosition { position: [ -1.0, -1.0, 0.5 ] },
                    ].iter().cloned()).expect("failed to create buffer");

                let ppvs = crate::shader::skybox::vertex::Shader::load(self.info.device.clone()).expect("failed to create shader module");
                let ppfs = crate::shader::skybox::fragment::Shader::load(self.info.device.clone()).expect("failed to create shader module");
                let temp_pipeline = Arc::new(GraphicsPipeline::start()
                    .cull_mode_disabled()
                    .vertex_input_single_buffer::<MeshVertex>()
                    .vertex_shader(ppvs.main_entry_point(), ())
                    .triangle_list()
                    .viewports_dynamic_scissors_irrelevant(1)
                    .fragment_shader(ppfs.main_entry_point(), ())
                    //.depth_stencil_simple_depth()
                    .blend_alpha_blending()
                    .render_pass(Subpass::from(self.stages.mesh_shading.get_renderpass().clone(), 0).unwrap())
                    .build(self.info.device.clone())
                    .unwrap());

                let vertgroup = skybox.vertex_groups[0].clone();
                let mut cb = AutoCommandBufferBuilder::primary_one_time_submit(self.info.device.clone(), self.info.queues.main.as_ref().unwrap().family())
                    .unwrap()
                    .begin_render_pass(fb.clone(), false, vec![[0.0,0.0,0.0,1.0].into()]).unwrap()
                    .draw_indexed(temp_pipeline.clone(), &DynamicState {
                            line_width: None,
                            viewports: Some(vec![Viewport {
                                origin: [0.0, 0.0],
                                dimensions: [self.info.dimensions[0] as f32, self.info.dimensions[1] as f32],
                                depth_range: 0.0..1.0,
                            }]),
                            scissors: None,
                            compare_mask: None,
                            write_mask: None,
                            reference: None
                        },
                                  vertgroup.vertex_buffer.clone(),
                                  vertgroup.index_buffer.clone(),
                                             (),
                                             // TODO: handle actual push constants
                                             crate::shader::skybox::vertex::ty::Constants {
                                                 matrix: (self.info.proj_mat.clone() * Matrix4::from(self.info.camera_transform.rotation)).into(),
                                                 sun_rotation: 0.0,
                                                 sun_transit: 0.4,
                                             }).unwrap()
                    .end_render_pass().unwrap()
                    .build().unwrap();

                let mut future = Box::new(vulkano::sync::now(self.info.device.clone()).then_execute(self.info.queues.main.as_ref().unwrap().clone(), cb).unwrap()
                    .then_signal_fence_and_flush().unwrap());

                self.info.mesh_queue.lock().clear();

                future
            }
        }
    }

    pub fn get_material(&self, name: &str) -> Option<&Arc<dyn MaterialDefinition + Send + Sync>> {
        self.info.materials.get(name)
    }
}
