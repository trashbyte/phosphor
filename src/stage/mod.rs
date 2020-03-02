use vulkano::pipeline::GraphicsPipelineAbstract;
use std::sync::Arc;
use vulkano::framebuffer::{RenderPassAbstract, FramebufferAbstract};
use vulkano::image::SwapchainImage;
use winit::Window;
use crate::renderer::RenderInfo;
use vulkano::command_buffer::{AutoCommandBuffer};
use vulkano::device::Queue;

pub mod mesh_shading;
pub mod resolve_scene_color;


//pub struct RenderStageDefinition {
//    pub device: Arc<Device>,
//    pub queue: Arc<Queue>,
//    pub pipeline: Arc<dyn GraphicsPipelineAbstract + Send + Sync>,
//}

pub trait RenderStageDefinition {
    fn get_pipeline(&self) -> &Arc<dyn GraphicsPipelineAbstract + Send + Sync>;
    fn get_renderpass(&self) -> &Arc<dyn RenderPassAbstract + Send + Sync>;
    fn get_framebuffers(&self) -> &Option< Vec<Arc<dyn FramebufferAbstract + Send + Sync>> >;
    fn get_framebuffers_mut(&mut self) -> &mut Option< Vec<Arc<dyn FramebufferAbstract + Send + Sync>> >;

    fn build_command_buffers(&mut self, info: &RenderInfo) -> Option<Vec<(AutoCommandBuffer, Arc<Queue>)>>;
    fn recreate_framebuffers_if_none(&mut self, images: &Vec<Arc<SwapchainImage<Window>>>, info: &RenderInfo);
}


// what does a render stage describe?
// unique shader type (remember there are ubershaders for common material types)
// attachment requirements and usage
// creates command buffers

// really just a light wrapper around a given pipeline, containing the necessary resources and
// descriptors to make it function
