use std::sync::Arc;
use vulkano::device::Device;
use vulkano::framebuffer::{Subpass, RenderPassAbstract};
use vulkano::pipeline::{GraphicsPipeline, GraphicsPipelineAbstract};
use vulkano::descriptor::DescriptorSet;

use crate::geometry::{MeshVertex, VertexPositionUV};
use crate::material::params::MaterialParams;
use vulkano::descriptor::descriptor_set::PersistentDescriptorSet;
use crate::renderer::RenderInfo;


pub mod params;


// Material Instances //////////////////////////////////////////////////////////////////////////////


/// An instance of a static material, i.e. one whose parameters are constant
#[derive(Clone)]
pub struct MaterialInstanceStatic {
    definition: Arc<dyn MaterialDefinition + Send + Sync>,
}
impl MaterialInstanceStatic {
    pub fn new(definition: Arc<dyn MaterialDefinition + Send + Sync>) -> Self {
        Self { definition }
    }
    pub fn descriptor_sets(&self) -> Vec<Arc<dyn DescriptorSet + Send + Sync>> {
        self.definition.static_descriptor_sets()
    }
    pub fn pipeline(&self) -> &Arc<dyn GraphicsPipelineAbstract + Send + Sync> {
        self.definition.pipeline()
    }
}

/// An instance of a dynamic material, i.e. one whose parameters are updated, potentially every frame
#[derive(Clone)]
pub struct MaterialInstanceDynamic {
    definition: Arc<dyn MaterialDefinition + Send + Sync>,
    cached_descriptor_sets: Vec<Arc<dyn DescriptorSet + Send + Sync>>,
    //cached_buffer: XallocCpuBufferPoolChunk<u8>,
    //buffer_pool: XallocCpuBufferPool<u8>,
}
impl MaterialInstanceDynamic {
    pub fn new(definition: Arc<dyn MaterialDefinition + Send + Sync>, params: MaterialParams) -> Self {
        Self {
            definition,
            cached_descriptor_sets: Vec::new(),
        }
    }
    pub fn descriptor_sets(&self) -> Vec<Arc<dyn DescriptorSet + Send + Sync>> {
        self.definition.static_descriptor_sets()
    }
    pub fn pipeline(&self) -> &Arc<dyn GraphicsPipelineAbstract + Send + Sync> {
        self.definition.pipeline()
    }
    pub fn update(&mut self) {

    }
}

#[derive(Clone)]
pub enum MaterialInstance {
    Static(MaterialInstanceStatic),
    Dynamic(MaterialInstanceDynamic),
}
impl MaterialInstance {
    pub fn pipeline(&self) -> &Arc<dyn GraphicsPipelineAbstract + Send + Sync> {
        match self {
            MaterialInstance::Static(inner) => inner.pipeline(),
            MaterialInstance::Dynamic(inner) => inner.pipeline(),
        }
    }
    pub fn descriptor_sets(&self) -> Vec<Arc<dyn DescriptorSet + Send + Sync>> {
        match self {
            MaterialInstance::Static(inner) => inner.descriptor_sets(),
            MaterialInstance::Dynamic(inner) => inner.descriptor_sets(),
        }
    }
}


// Material Definitions ////////////////////////////////////////////////////////////////////////////


pub trait MaterialDefinition {
    fn pipeline(&self) -> &Arc<dyn GraphicsPipelineAbstract + Send + Sync>;
    fn params_accepted(&self) -> MaterialParams;
    fn static_descriptor_sets(&self) -> Vec<Arc<dyn DescriptorSet + Send + Sync>> { Vec::new() }
}


// Material Implementations ////////////////////////////////////////////////////////////////////////


pub struct GenericMeshMaterial {
    pipeline: Arc<dyn GraphicsPipelineAbstract + Send + Sync>,
    static_descriptor_sets: Vec<Arc<dyn DescriptorSet + Send + Sync>>,
}

impl GenericMeshMaterial {
    pub fn new(device: Arc<Device>, pass: Arc<dyn RenderPassAbstract + Send + Sync>, subpass: u32) -> Self {
        let vs = crate::shader::mesh_generic::vertex::Shader::load(device.clone()).expect("failed to create shader module");
        let fs = crate::shader::mesh_generic::fragment::Shader::load(device.clone()).expect("failed to create shader module");
        let pipeline = Arc::new(GraphicsPipeline::start()
            .cull_mode_back()
            .vertex_input_single_buffer::<MeshVertex>()
            .vertex_shader(vs.main_entry_point(), ())
            .triangle_list()
            .viewports_dynamic_scissors_irrelevant(1)
            .fragment_shader(fs.main_entry_point(), ())
            //.depth_stencil_simple_depth()
            .render_pass(Subpass::from(pass, subpass).unwrap())
            .build(device.clone())
            .unwrap());

//        let pbr_texture_descriptors = Arc::new(PersistentDescriptorSet::start(pipeline.clone(), 0)
//           .add_sampled_image(tex_registry.get("grass").unwrap().clone(), linear_sampler.clone()).unwrap()
//           .add_sampled_image(tex_registry.get("test_normal").unwrap().clone(), linear_sampler.clone()).unwrap()
//           .add_sampled_image(tex_registry.get("black").unwrap().clone(), linear_sampler.clone()).unwrap()
//           .add_sampled_image(tex_registry.get("black").unwrap().clone(), linear_sampler.clone()).unwrap()
//       .build().unwrap());

        Self { pipeline, static_descriptor_sets: vec![ ] }
    }
}

impl MaterialDefinition for GenericMeshMaterial {
    fn pipeline(&self) -> &Arc<dyn GraphicsPipelineAbstract + Send + Sync> {
        &self.pipeline
    }

    fn params_accepted(&self) -> MaterialParams {
        unimplemented!()
    }

    fn static_descriptor_sets(&self) -> Vec<Arc<dyn DescriptorSet + Send + Sync>> {
        self.static_descriptor_sets.clone()
    }
}

pub struct SkyboxMaterial {
    pipeline: Arc<dyn GraphicsPipelineAbstract + Send + Sync>,
    static_descriptor_sets: Vec<Arc<dyn DescriptorSet + Send + Sync>>
}

impl SkyboxMaterial {
    pub fn new(info: &RenderInfo, pass: Arc<dyn RenderPassAbstract + Send + Sync>, subpass: u32, params: MaterialParams) -> Self {
        let vs = crate::shader::skybox::vertex::Shader::load(info.device.clone()).expect("failed to create shader module");
        let fs = crate::shader::skybox::fragment::Shader::load(info.device.clone()).expect("failed to create shader module");
        let pipeline = Arc::new(GraphicsPipeline::start()
            .cull_mode_disabled()
            .vertex_input_single_buffer::<MeshVertex>()
            .vertex_shader(vs.main_entry_point(), ())
            .triangle_list()
            .viewports_dynamic_scissors_irrelevant(1)
            .fragment_shader(fs.main_entry_point(), ())
            //.depth_stencil_simple_depth()
            .blend_alpha_blending()
            .render_pass(Subpass::from(pass, subpass).unwrap())
            .build(info.device.clone())
            .unwrap());

        Self { pipeline, static_descriptor_sets: vec![ ] }
    }
}

impl MaterialDefinition for SkyboxMaterial {
    fn pipeline(&self) -> &Arc<dyn GraphicsPipelineAbstract + Send + Sync> { &self.pipeline }

    fn params_accepted(&self) -> MaterialParams {
        unimplemented!()
    }

    fn static_descriptor_sets(&self) -> Vec<Arc<dyn DescriptorSet + Send + Sync>> {
        self.static_descriptor_sets.clone()
    }
}
