use std::sync::Arc;
use parking_lot::Mutex;
use crate::buffer::CpuAccessibleBufferXalloc;
use vulkano::buffer::BufferUsage;
use vulkano::pipeline::{ComputePipeline, ComputePipelineAbstract};
use vulkano::descriptor::DescriptorSet;
use vulkano::descriptor::descriptor_set::PersistentDescriptorSet;
use vulkano::command_buffer::AutoCommandBufferBuilder;
use vulkano::device::{Device, Queue};
use vulkano::sync::GpuFuture;
use std::sync::atomic::{AtomicBool, Ordering};


// 1024 x 768 -> 786432
//
// stage1 -> 786432 / 128 -> 6144
// stage2 -> 8192 / 128 -> 64
//
// cpu sums 64 floats


lazy_static! {
    pub static ref REDUCTION_SOLVER_WORKING: AtomicBool = AtomicBool::new(false);
}


pub struct ParallelReductionSolver {
    pub pipeline: Arc<dyn ComputePipelineAbstract + Send + Sync>,
    pub source_buffer: Arc<CpuAccessibleBufferXalloc<[u16]>>,
    pub intermediate_buffer: Arc<CpuAccessibleBufferXalloc<[u16]>>,
    pub dest_buffer: Arc<CpuAccessibleBufferXalloc<[u16]>>,
    pub stage1_ds: Arc<dyn DescriptorSet + Send + Sync>,
    pub stage2_ds: Arc<dyn DescriptorSet + Send + Sync>,
    pub avg: Arc<Mutex<f32>>
}

impl ParallelReductionSolver {
    pub fn new(device: Arc<Device>) -> Self {
        let pipeline = Arc::new({
            let shader = crate::shader::reduction::Shader::load(device.clone()).unwrap();
            ComputePipeline::new(device.clone(), &shader.main_entry_point(), &()).unwrap()
        });

        let storage_buf_usage = BufferUsage {
            storage_buffer: true,
            transfer_destination: true,
            transfer_source: true,
            ..BufferUsage::none()
        };

        let source_buffer = CpuAccessibleBufferXalloc::from_iter(device.clone(),  storage_buf_usage.clone(), [0u16; 1024*768*4].iter().cloned()).unwrap();
        let intermediate_buffer = CpuAccessibleBufferXalloc::from_iter(device.clone(), storage_buf_usage.clone(), [0u16; 6144].iter().cloned()).unwrap();
        let dest_buffer = CpuAccessibleBufferXalloc::from_iter(device.clone(), storage_buf_usage.clone(), [0u16; 64].iter().cloned()).unwrap();

        let stage1_ds = Arc::new(PersistentDescriptorSet::start(pipeline.clone(), 0)
            .add_buffer(source_buffer.clone()).unwrap()
            .add_buffer(dest_buffer.clone()).unwrap() // fuck
            .build().unwrap()
        );

        let stage2_ds = Arc::new(PersistentDescriptorSet::start(pipeline.clone(), 0)
            .add_buffer(intermediate_buffer.clone()).unwrap()
            .add_buffer(dest_buffer.clone()).unwrap()
            .build().unwrap()
        );

        Self {
            pipeline,
            source_buffer,
            intermediate_buffer,
            dest_buffer,
            stage1_ds,
            stage2_ds,
            avg: Arc::new(Mutex::new(0.0))
        }
    }

    // blocks until execution is finished, so call on another thread
    pub fn submit(&self, device: Arc<Device>, queue: Arc<Queue>) {
        REDUCTION_SOLVER_WORKING.store(true, Ordering::Relaxed);
        let stage1_cb = AutoCommandBufferBuilder::primary_one_time_submit(device.clone(), queue.family()).unwrap()
            .dispatch([1, 1, 1], self.pipeline.clone(), self.stage1_ds.clone(), ()).unwrap()
            .build().unwrap();
//        let stage2_cb = AutoCommandBufferBuilder::primary_one_time_submit(device.clone(), queue.family()).unwrap()
//            .dispatch([64, 1, 1], self.pipeline.clone(), self.stage2_ds.clone(), ()).unwrap()
//            .build().unwrap();
        let future = vulkano::sync::now(device.clone()).then_execute(queue.clone(), stage1_cb);
        match future {
            Err(e) => {
                println!("Error in luma reduction compute stage 1: {}", e);
                REDUCTION_SOLVER_WORKING.store(false, Ordering::Relaxed);
                return;
            }
            _ => {}
        }
//        let future = future.unwrap().then_signal_semaphore_and_flush().unwrap()
//            .then_execute(queue.clone(), stage2_cb);
//        match future {
//            Err(e) => {
//                error!(Test, "Error in luma reduction compute stage 2: {}", e);
//                REDUCTION_SOLVER_WORKING.store(false, Ordering::Relaxed);
//                return;
//            }
//            _ => {}
//        }
        let future = future.unwrap().then_signal_fence_and_flush().unwrap();
        future.wait(None).unwrap();
        {
            let lock = self.dest_buffer.read().unwrap();
            let sum = half::f16::from_bits(lock[0] as u16).to_f32();
            println!("luma sum: {}", sum / 786432.0);
            *self.avg.lock() = sum / 786432.0;
        }

        REDUCTION_SOLVER_WORKING.store(false, Ordering::Relaxed);
    }
}
