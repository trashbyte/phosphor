//! Custom RenderPass types.

pub mod mesh_shading;
pub use self::mesh_shading::GenericMeshShadingRenderPass;

pub mod deferred_lighting;
pub use self::deferred_lighting::DeferredLightingRenderPass;

pub mod lines;
pub use self::lines::LinesRenderPass;

pub mod occlusion;
pub use self::occlusion::OcclusionRenderPass;

pub mod resolve_scene_color;
pub use self::resolve_scene_color::ResolveSceneColorRenderPass;
