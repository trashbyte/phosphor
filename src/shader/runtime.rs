use std::fs::File;
use std::io::Read;
use std::ffi::CStr;
use std::sync::Arc;

use vulkano::pipeline::shader::{ShaderInterfaceDef, ShaderInterfaceDefEntry, ShaderModule, GraphicsShaderType};
use vulkano::format::Format;
use vulkano::descriptor::descriptor::{ShaderStages, DescriptorDesc};
use vulkano::descriptor::pipeline_layout::{PipelineLayoutDesc, PipelineLayoutDescPcRange};
use vulkano::pipeline::vertex::SingleBufferDefinition;
use vulkano::descriptor::PipelineLayoutAbstract;
use vulkano::pipeline::GraphicsPipeline;
use vulkano::device::Device;
use vulkano::framebuffer::{Subpass, RenderPassAbstract};

use crate::geometry::MeshVertex;

#[derive(Debug, Clone)]
enum InterfaceParameter { Float, Vec2, Vec3, Vec4 }
impl InterfaceParameter {
    pub fn format(&self) -> Format {
        match self {
            InterfaceParameter::Float => Format::R32Sfloat,
            InterfaceParameter::Vec2  => Format::R32G32Sfloat,
            InterfaceParameter::Vec3  => Format::R32G32B32Sfloat,
            InterfaceParameter::Vec4  => Format::R32G32B32A32Sfloat,
        }
    }
}


// Vertex stage ////////////////////////////////////////////////////////////////////////////////////


#[derive(Debug, Clone)]
struct VertInput(Vec<(String, InterfaceParameter)>);
unsafe impl ShaderInterfaceDef for VertInput {
    type Iter = VertInputIter;

    fn elements(&self) -> VertInputIter {
        VertInputIter {
            elements: self.0.clone(),
            position: 0
        }
    }
}

#[derive(Debug, Clone)]
struct VertInputIter {
    elements: Vec<(String, InterfaceParameter)>,
    position: usize,
}
impl Iterator for VertInputIter {
    type Item = ShaderInterfaceDefEntry;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.position >= self.elements.len() { None }
        else {
            let (_, param) = self.elements[self.position].clone();
            let result = Some(ShaderInterfaceDefEntry {
                location: (self.position as u32)..(self.position as u32 + 1),
                format: param.format(),
                name: None, // TODO: parameter names (?)
            });
            self.position += 1;
            result
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.elements.len() - self.position;
        (len, Some(len))
    }
}
impl ExactSizeIterator for VertInputIter { }

#[derive(Debug, Clone)]
struct VertOutput(Vec<(String, InterfaceParameter)>);

unsafe impl ShaderInterfaceDef for VertOutput {
    type Iter = VertOutputIter;

    fn elements(&self) -> VertOutputIter {
        VertOutputIter {
            elements: self.0.clone(),
            position: 0
        }
    }
}

#[derive(Debug, Clone)]
struct VertOutputIter {
    elements: Vec<(String, InterfaceParameter)>,
    position: usize,
}

impl Iterator for VertOutputIter {
    type Item = ShaderInterfaceDefEntry;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.position >= self.elements.len() { None }
        else {
            let (_, param) = self.elements[self.position].clone();
            let result = Some(ShaderInterfaceDefEntry {
                location: (self.position as u32)..(self.position as u32 + 1),
                format: param.format(),
                name: None,
            });
            self.position += 1;
            result
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.elements.len() - self.position;
        (len, Some(len))
    }
}

impl ExactSizeIterator for VertOutputIter { }

#[derive(Debug, Clone)]
struct VertLayout(ShaderStages);

unsafe impl PipelineLayoutDesc for VertLayout {
    // Number of descriptor sets it takes.
    fn num_sets(&self) -> usize { 0 }
    // Number of entries (bindings) in each set.
    fn num_bindings_in_set(&self, _set: usize) -> Option<usize> { None }
    // Descriptor descriptions.
    fn descriptor(&self, _set: usize, _binding: usize) -> Option<DescriptorDesc> { None }
    // Number of push constants ranges (think: number of push constants).
    fn num_push_constants_ranges(&self) -> usize { 0 }
    // Each push constant range in memory.
    fn push_constants_range(&self, _num: usize) -> Option<PipelineLayoutDescPcRange> { None }
}


// Fragment stage //////////////////////////////////////////////////////////////////////////////////


#[derive(Debug, Clone)]
struct FragInput(Vec<(String, InterfaceParameter)>);

unsafe impl ShaderInterfaceDef for FragInput {
    type Iter = FragInputIter;

    fn elements(&self) -> FragInputIter {
        FragInputIter {
            elements: self.0.clone(),
            position: 0
        }
    }
}

#[derive(Debug, Clone)]
struct FragInputIter {
    elements: Vec<(String, InterfaceParameter)>,
    position: usize,
}

impl Iterator for FragInputIter {
    type Item = ShaderInterfaceDefEntry;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.position >= self.elements.len() { None }
        else {
            let (_, param) = self.elements[self.position].clone();
            let result = Some(ShaderInterfaceDefEntry {
                location: (self.position as u32)..(self.position as u32 + 1),
                format: param.format(),
                name: None,
            });
            self.position += 1;
            result
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.elements.len() - self.position;
        (len, Some(len))
    }
}

impl ExactSizeIterator for FragInputIter { }

#[derive(Debug, Clone)]
struct FragOutput(Vec<(String, InterfaceParameter)>);

unsafe impl ShaderInterfaceDef for FragOutput {
    type Iter = FragOutputIter;

    fn elements(&self) -> FragOutputIter {
        FragOutputIter {
            elements: self.0.clone(),
            position: 0
        }
    }
}

#[derive(Debug, Clone)]
struct FragOutputIter {
    elements: Vec<(String, InterfaceParameter)>,
    position: usize,
}

impl Iterator for FragOutputIter {
    type Item = ShaderInterfaceDefEntry;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.position >= self.elements.len() { None }
        else {
            let (_, param) = self.elements[self.position].clone();
            let result = Some(ShaderInterfaceDefEntry {
                location: (self.position as u32)..(self.position as u32 + 1),
                format: param.format(),
                name: None,
            });
            self.position += 1;
            result
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.elements.len() - self.position;
        (len, Some(len))
    }
}

impl ExactSizeIterator for FragOutputIter { }

#[derive(Debug, Clone)]
struct FragLayout(ShaderStages);

unsafe impl PipelineLayoutDesc for FragLayout {
    fn num_sets(&self) -> usize { 0 }
    fn num_bindings_in_set(&self, _set: usize) -> Option<usize> { None }
    fn descriptor(&self, _set: usize, _binding: usize) -> Option<DescriptorDesc> { None }
    fn num_push_constants_ranges(&self) -> usize { 0 }
    fn push_constants_range(&self, _num: usize) -> Option<PipelineLayoutDescPcRange> { None }
}


// Public API //////////////////////////////////////////////////////////////////////////////////////


pub fn build_shader_pipeline(vert_path: &str, frag_path: &str, device: Arc<Device>, pass: Arc<dyn RenderPassAbstract + Send + Sync>, subpass: u32) -> Arc<dyn PipelineLayoutAbstract + Send + Sync> {
    let vs = {
        let mut f = File::open(vert_path)
            .expect(&format!("Can't find file '{}'", vert_path));
        let mut v = vec![];
        f.read_to_end(&mut v).unwrap();
        // TODO: correctness checks
        unsafe { ShaderModule::new(device.clone(), &v) }.unwrap()
    };

    let fs = {
        let mut f = File::open(frag_path)
            .expect(&format!("Can't find file '{}'", frag_path));
        let mut v = vec![];
        f.read_to_end(&mut v).unwrap();
        // TODO: correctness checks
        unsafe { ShaderModule::new(device.clone(), &v) }.unwrap()
    };

    let vert_inputs = vec![
        ("position".to_string(), InterfaceParameter::Vec3),
        ("uv".to_string(), InterfaceParameter::Vec2),
    ];
    let vert_outputs = vec![
        ("out_uv".to_string(), InterfaceParameter::Vec2),
    ];
    let frag_inputs = vec![
        ("uv".to_string(), InterfaceParameter::Vec2),
    ];
    let frag_outputs = vec![
        ("outFragColor".to_string(), InterfaceParameter::Vec4),
    ];

    let vert_main = unsafe { vs.graphics_entry_point(
        CStr::from_bytes_with_nul_unchecked(b"main\0"),
        VertInput(vert_inputs),
        VertOutput(vert_outputs),
        VertLayout(ShaderStages { vertex: true, ..ShaderStages::none() }),
        GraphicsShaderType::Vertex
    ) };

    let frag_main = unsafe { fs.graphics_entry_point(
        CStr::from_bytes_with_nul_unchecked(b"main\0"),
        FragInput(frag_inputs),
        FragOutput(frag_outputs),
        FragLayout(ShaderStages { fragment: true, ..ShaderStages::none() }),
        GraphicsShaderType::Fragment
    ) };

    Arc::new(GraphicsPipeline::start()
                 .vertex_input(SingleBufferDefinition::<MeshVertex>::new())
                 .vertex_shader(vert_main, ())
                 .triangle_list()
                 .viewports_dynamic_scissors_irrelevant(1)
                 .fragment_shader(frag_main, ())
                 .depth_stencil_simple_depth()
                 .render_pass(Subpass::from(pass, subpass).unwrap())
                 .build(device.clone())
                 .unwrap(),
    )
}
