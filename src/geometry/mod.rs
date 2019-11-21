//! Geometry related types.
//!
//! Includes mesh, vertex, and vertexgroup types. `Material` is here too, because I couldn't find a
//! better place for it.


pub mod mesh;
pub mod vertex;
pub mod vertexgroup;

pub use self::mesh::Mesh;
pub use self::vertex::{VertexPositionColorAlpha, VertexPosition, DeferredShadingVertex, VertexPositionObjectId, VertexPositionUV};
pub use self::vertexgroup::VertexGroup;


/// Shader parameters for a given material.
#[derive(Clone, Debug)]
pub struct Material {
    /// Name of albedo map, used to look up texture in the [TextureRegistry](::registry::TextureRegistry).
    pub albedo_map_name: String,
    /// Exponent used in specular lighting calculation. Higher values have sharper highlights.
    pub specular_exponent: f32,
    /// Intensity of specular highlights.
    pub specular_strength: f32
}


pub mod cube {
    use crate::geometry::VertexPositionColorAlpha;


    // TODO: chnage to generic "draw box"
    pub fn generate_chunk_debug_line_vertices(x: i32, y: i32, z: i32, size: f32, alpha: f32) -> [VertexPositionColorAlpha; 8] {
        let x = x as f32 * size;
        let y = y as f32 * size;
        let z = z as f32 * size;
        [
            // top
            VertexPositionColorAlpha { position: [ x,       y+ size, z+ size], color: [ 1.0, 1.0, 1.0, alpha ] },
            VertexPositionColorAlpha { position: [ x+ size, y+ size, z+ size], color: [ 1.0, 1.0, 1.0, alpha ] },
            VertexPositionColorAlpha { position: [ x+ size, y+ size, z      ], color: [ 1.0, 1.0, 1.0, alpha ] },
            VertexPositionColorAlpha { position: [ x,       y+ size, z      ], color: [ 1.0, 1.0, 1.0, alpha ] },
            // bottom
            VertexPositionColorAlpha { position: [ x,       y, z+ size], color: [ 1.0, 1.0, 1.0, alpha ] },
            VertexPositionColorAlpha { position: [ x+ size, y, z+ size], color: [ 1.0, 1.0, 1.0, alpha ] },
            VertexPositionColorAlpha { position: [ x+ size, y, z      ], color: [ 1.0, 1.0, 1.0, alpha ] },
            VertexPositionColorAlpha { position: [ x,       y, z      ], color: [ 1.0, 1.0, 1.0, alpha ] },
        ]
    }


    pub fn generate_chunk_debug_line_indices(offset: u32) -> [u32; 24] {
        let o = offset * 8;
        [
            0+o,  1+o,  1+o,  2+o,  2+o,  3+o, 3+o, 0+o, // top
            0+o,  4+o,  1+o,  5+o,  2+o,  6+o, 3+o, 7+o, // middle
            4+o,  5+o,  5+o,  6+o,  6+o,  7+o, 7+o, 4+o, // bottom
        ]
    }
}