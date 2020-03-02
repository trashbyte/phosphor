use vulkano::framebuffer::{RenderPassDesc, AttachmentDescription, PassDescription, PassDependencyDescription, LoadOp, StoreOp, RenderPassDescClearValues};
use vulkano::image::ImageLayout;
use vulkano::format::{Format, ClearValue};
use vulkano::sync::{PipelineStages, AccessFlagBits};


/// Render pass for post processing.
pub struct ResolveSceneColorRenderPass;

const DIFFUSE_IN:  usize = 0;
const SPECULAR_IN: usize = 1;
const SCENE_COLOR:   usize = 2;
const LUMA_BUFFER:   usize = 3;

const FLOAT_INPUT: AttachmentDescription = AttachmentDescription {
    format: Format::R16G16B16A16Sfloat,
    samples: 1,
    load: LoadOp::Load,
    store: StoreOp::DontCare,
    stencil_load: LoadOp::DontCare,
    stencil_store: StoreOp::DontCare,
    initial_layout: ImageLayout::Undefined,
    final_layout: ImageLayout::ColorAttachmentOptimal
};

unsafe impl RenderPassDesc for ResolveSceneColorRenderPass {
    fn num_attachments(&self) -> usize { 4 }
    fn attachment_desc(&self, num: usize) -> Option<AttachmentDescription> {
        match num {
            DIFFUSE_IN => Some(FLOAT_INPUT),
            SPECULAR_IN => Some(FLOAT_INPUT),
            SCENE_COLOR => Some(AttachmentDescription {
                format: Format::R16G16B16A16Sfloat,
                samples: 1,
                load: LoadOp::Clear,
                store: StoreOp::Store,
                stencil_load: LoadOp::DontCare,
                stencil_store: StoreOp::DontCare,
                initial_layout: ImageLayout::ColorAttachmentOptimal,
                final_layout: ImageLayout::ColorAttachmentOptimal
            }),
            LUMA_BUFFER => Some(AttachmentDescription {
                format: Format::R32Uint,
                samples: 1,
                load: LoadOp::Clear,
                store: StoreOp::Store,
                stencil_load: LoadOp::DontCare,
                stencil_store: StoreOp::DontCare,
                initial_layout: ImageLayout::Undefined,
                final_layout: ImageLayout::ColorAttachmentOptimal
            }),
            _ => None
        }
    }

    fn num_subpasses(&self) -> usize { 1 }
    fn subpass_desc(&self, num: usize) -> Option<PassDescription> {
        match num {
            0 => Some(PassDescription {
                color_attachments: vec![
                    (SCENE_COLOR, ImageLayout::ColorAttachmentOptimal),
                    (LUMA_BUFFER, ImageLayout::ColorAttachmentOptimal)
                ],
                depth_stencil: None,
                input_attachments: vec![
                    (DIFFUSE_IN, ImageLayout::ColorAttachmentOptimal),
                    (SPECULAR_IN, ImageLayout::ColorAttachmentOptimal)
                ],
                resolve_attachments: vec![],
                preserve_attachments: vec![]
            }),
            _ => None
        }
    }

    fn num_dependencies(&self) -> usize { 1 }
    fn dependency_desc(&self, num: usize) -> Option<PassDependencyDescription> {
        match num {
            0 => {
                Some(PassDependencyDescription {
                    source_subpass: 0xffffffff,
                    destination_subpass: 0,
                    source_stages: PipelineStages {
                        color_attachment_output: true,
                        ..PipelineStages::none()
                    },
                    destination_stages: PipelineStages {
                        fragment_shader: true,
                        ..PipelineStages::none()
                    },
                    source_access: AccessFlagBits {
                        index_read: true,
                        vertex_attribute_read: true,
                        color_attachment_write: true,
                        depth_stencil_attachment_write: true,
                        memory_read: true,
                        memory_write: true,
                        ..AccessFlagBits::none()
                    },
                    destination_access: AccessFlagBits {
                        index_read: true,
                        vertex_attribute_read: true,
                        input_attachment_read: true,
                        color_attachment_read: true,
                        color_attachment_write: true,
                        depth_stencil_attachment_read: true,
                        depth_stencil_attachment_write: true,
                        memory_read: true,
                        memory_write: true,
                        ..AccessFlagBits::none()
                    },
                    by_region: false
                })
            },
            _ => None
        }
    }
}


unsafe impl RenderPassDescClearValues<Vec<ClearValue>> for ResolveSceneColorRenderPass {
    fn convert_clear_values(&self, values: Vec<ClearValue>) -> Box<dyn Iterator<Item = ClearValue>> {
        // FIXME: safety checks
        Box::new(values.into_iter())
    }
}
