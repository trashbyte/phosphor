use std::sync::Arc;
use crate::buffer::CpuAccessibleBufferXalloc;
use vulkano::buffer::BufferUsage;
use vulkano::pipeline::{ComputePipeline, ComputePipelineAbstract};
use vulkano::descriptor::DescriptorSet;
use vulkano::descriptor::descriptor_set::PersistentDescriptorSet;
use vulkano::command_buffer::AutoCommandBufferBuilder;
use vulkano::device::{Device, Queue};
use vulkano::sync::GpuFuture;
use std::sync::atomic::{AtomicBool, Ordering};


lazy_static! {
    pub static ref HISTOGRAM_COMPUTE_WORKING: AtomicBool = AtomicBool::new(false);
}


pub struct HistogramCompute {
    pub pipeline: Arc<dyn ComputePipelineAbstract + Send + Sync>,
    pub source_buffer: Arc<CpuAccessibleBufferXalloc<[u32]>>,
    pub bins_buffer: Arc<CpuAccessibleBufferXalloc<[u32]>>,
    pub desc_set: Arc<dyn DescriptorSet + Send + Sync>,
    pub bins: [u32; 128],
    pub low_percentile_bin: f32,
    pub high_percentile_bin: f32,
}

impl HistogramCompute {
    pub fn new(device: Arc<Device>) -> Self {
        let pipeline = Arc::new({
            let shader = crate::shader::histogram::Shader::load(device.clone()).unwrap();
            ComputePipeline::new(device.clone(), &shader.main_entry_point(), &()).unwrap()
        });

        let storage_buf_usage = BufferUsage {
            storage_buffer: true,
            transfer_destination: true,
            transfer_source: true,
            ..BufferUsage::none()
        };

        let source_buffer = CpuAccessibleBufferXalloc::from_iter(device.clone(),  storage_buf_usage.clone(), [0u32; 512*512].iter().cloned()).unwrap();
        let bins_buffer = CpuAccessibleBufferXalloc::from_iter(device.clone(), storage_buf_usage.clone(), [0u32; 128].iter().cloned()).unwrap();

        let desc_set = Arc::new(PersistentDescriptorSet::start(pipeline.clone(), 0)
            .add_buffer(source_buffer.clone()).unwrap()
            .add_buffer(bins_buffer.clone()).unwrap()
            .build().unwrap()
        );

        Self {
            pipeline,
            source_buffer,
            bins_buffer,
            desc_set,
            bins: [0u32; 128],
            low_percentile_bin: 0.0,
            high_percentile_bin: 127.0,
        }
    }

    // blocks until execution is finished, so call on another thread
    pub fn submit(&mut self, device: Arc<Device>, queue: Arc<Queue>) {
        HISTOGRAM_COMPUTE_WORKING.store(true, Ordering::Relaxed);
        {
            let mut lock = self.bins_buffer.write().unwrap();
            for b in lock.iter_mut() {
                *b = 0;
            }
        }
        let cb = AutoCommandBufferBuilder::primary_one_time_submit(device.clone(), queue.family()).unwrap()
            .dispatch([16, 1, 1], self.pipeline.clone(), self.desc_set.clone(), ()).unwrap()
            .build().unwrap();
        let future = vulkano::sync::now(device.clone()).then_execute(queue.clone(), cb);
        match future {
            Err(e) => {
                println!("Error in histogram compute: {}", e);
                HISTOGRAM_COMPUTE_WORKING.store(false, Ordering::Relaxed);
                return;
            }
            _ => {}
        }
        let future = future.unwrap().then_signal_fence_and_flush().unwrap();
        future.wait(None).unwrap();
        {
            let lock = self.bins_buffer.read().unwrap();
            let mut counted = 0;
            let mut low_found = false;
            let mut high_found = false;
            for (i, b) in lock.iter().enumerate() {
                self.bins[i] = *b;
                counted += *b;
                if !low_found && counted >= 157286 { // 60%
                    // find how far through the bin the threshold is
                    let bin_begin = counted - *b;
                    let overshoot = 157286 - bin_begin;
                    let depth = overshoot as f32 / *b as f32;
                    // store value as (decimal) number of bins
                    self.low_percentile_bin = i as f32 + depth;
                    low_found = true;
                }
                if !high_found && counted >= 235930 { // 90%
                    // find how far through the bin the threshold is
                    let bin_begin = counted - *b;
                    let overshoot = 235930 - bin_begin;
                    let depth = overshoot as f32 / *b as f32;
                    // store value as (decimal) number of bins
                    self.high_percentile_bin = i as f32 + depth;
                    high_found = true;
                }
            }
        }

        HISTOGRAM_COMPUTE_WORKING.store(false, Ordering::Relaxed);
    }
}
