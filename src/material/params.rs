use std::sync::Arc;
use cgmath::Matrix4;
use vulkano::image::AttachmentImage;
use vulkano::format::R8G8B8A8Srgb;
use vulkano::sampler::Sampler;
use hashbrown::HashMap;
use vulkano::descriptor::pipeline_layout::PipelineLayoutDesc;


#[derive(Debug, Clone)]
pub enum MaterialParam {
    Float(f32),
    Vec2(f32, f32),
    Vec3(f32, f32, f32),
    Vec4(f32, f32, f32, f32),
    Mat4(Matrix4<f32>),
    Texture(Arc<AttachmentImage<R8G8B8A8Srgb>>, Arc<Sampler>)
}

#[derive(Debug, Clone)]
pub enum MaterialParamType { Float, Vec2, Vec3, Vec4, Mat4, Texture }


#[derive(Debug, Clone)]
pub struct MaterialParams {
    params: HashMap<String, MaterialParam>,
}

impl MaterialParams {
    pub fn new() -> Self {
        Self {
            params: HashMap::new(),
        }
    }
    pub fn from(params: HashMap<String, MaterialParam>) -> Self {
        Self {
            params,
        }
    }
    pub fn add(&mut self, name: &str, param: MaterialParam) {
        self.params.insert(name.to_string(), param);
    }
    pub fn get(&self, key: &str) -> Option<&MaterialParam> {
        self.params.get(key)
    }
    pub fn generate_descriptor_set<L>(&self, layout: L)// -> Arc<dyn DescriptorSet + Send + Sync>
        where L: PipelineLayoutDesc {

        //let mut builder = Box::new(PersistentDescriptorSet::start(layout, 0));



        //Arc::new(builder.build().unwrap())
    }
}
