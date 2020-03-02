//! A mesh object, made up of a set of vertex groups, a list of associated materials, and a transform.
//!
//! The vertgroup / material separation is necessary because a set of geometry can only be rendered
//! with one material at a time, so meshes with multiple materials are broken into multiple vertex groups.

use std::sync::Arc;

use crate::geometry::{VertexGroup, MeshVertex};
use toolbelt::Transform;


/// A mesh object, made up of a set of vertex groups, a list of associated materials, and a transform.
///
/// See [module-level documentation](self).
#[derive(Clone)]
pub struct Mesh {
    pub transform: Transform,
    pub vertex_groups: Vec<Arc<VertexGroup<MeshVertex>>>,
}


impl Mesh {
    /// Creates a new mesh with an identity transform and no geometry or materials.
    pub fn new() -> Mesh {
        Mesh {
            transform: Transform::identity(),
            vertex_groups: Vec::new(),
        }
    }
}
