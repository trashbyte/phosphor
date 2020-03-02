//! A vertex group type, which holds vertex and index buffers and a material.

use std::sync::Arc;

use vulkano::buffer::BufferUsage;
use vulkano::device::Device;

use crate::buffer::CpuAccessibleBufferXalloc;
use crate::material::MaterialInstance;


/// Vertex group object.
pub struct VertexGroup<V> {
    pub vertex_buffer: Arc<CpuAccessibleBufferXalloc<[V]>>,
    pub index_buffer: Arc<CpuAccessibleBufferXalloc<[u32]>>,
    pub material: MaterialInstance,
}


impl<V> VertexGroup<V> {
    /// Constructs a new `VertexGroup` with the given parameters.
    pub fn new<Iv, Ii>(verts: Iv, idxs: Ii, material: MaterialInstance, device: Arc<Device>) -> Arc<VertexGroup<V>>
            where Iv: ExactSizeIterator<Item=V>, Ii: ExactSizeIterator<Item=u32>, V: 'static {
        Arc::new(VertexGroup {
            vertex_buffer: CpuAccessibleBufferXalloc::from_iter(device.clone(), BufferUsage::all(), verts).expect("failed to create vertex buffer"),
            index_buffer: CpuAccessibleBufferXalloc::from_iter(device.clone(), BufferUsage::all(), idxs).expect("failed to create index buffer"),
            material
        })
    }
}
