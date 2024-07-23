use std::{ops::Range, sync::Arc};

use debug_helper::is_debug;
use vulkan_instance::VulkanInstance;
use vulkano::{
    command_buffer::{
        allocator::StandardCommandBufferAllocator, AutoCommandBufferBuilder,
        PrimaryAutoCommandBuffer,
    },
    query::{QueryPool, QueryPoolCreateInfo, QueryResultFlags},
    sync,
};

struct Inner {
    query_pool: Arc<QueryPool>,
}

/// A helper struct for handling vulkan timestamps.
/// This will only function in debug.
/// This allows for the timestamp pool to only be functional in, while not requiring any API changes.
pub struct TimestampPool(Option<Inner>);

impl TimestampPool {
    /// Panics if in debug  and any creation steps fail.
    pub fn new(vk: &VulkanInstance, timestamps: u32) -> Self {
        if !is_debug() || !vk.supports_timestamps() {
            return Self(None);
        }

        let mut query_info = QueryPoolCreateInfo::query_type(vulkano::query::QueryType::Timestamp);
        query_info.query_count = timestamps;

        let query_pool =
            QueryPool::new(vk.device.clone(), query_info).expect("Failed to create query pool");

        Self(Some(Inner { query_pool }))
    }

    pub fn reset_timestamps(
        &self,
        builder: &mut AutoCommandBufferBuilder<
            PrimaryAutoCommandBuffer<Arc<StandardCommandBufferAllocator>>,
            Arc<StandardCommandBufferAllocator>,
        >,
        timestamps: Range<u32>,
    ) {
        let Some(Inner { query_pool }) = self.0.as_ref() else {
            return;
        };

        unsafe {
            builder
                .reset_query_pool(query_pool.clone(), timestamps)
                .expect("Failed to reset query pool");
        }
    }

    pub fn record_timestamp(
        &self,
        builder: &mut AutoCommandBufferBuilder<
            PrimaryAutoCommandBuffer<Arc<StandardCommandBufferAllocator>>,
            Arc<StandardCommandBufferAllocator>,
        >,
        timestamp: u32,
        stage: sync::PipelineStage,
    ) {
        let Some(Inner { query_pool }) = self.0.as_ref() else {
            return;
        };

        unsafe {
            builder
                .write_timestamp(query_pool.clone(), timestamp, stage)
                .expect("Failed to write timestamp");
        };
    }

    pub fn get_delta_ms_available(
        &self,
        vk: &VulkanInstance,
        timestamps: Range<u32>,
    ) -> Option<f64> {
        let Inner { query_pool } = self.0.as_ref()?;

        let mut results: Vec<u64> = vec![0; timestamps.len() * 2];
        query_pool
            .get_results(
                timestamps.clone(),
                &mut results,
                QueryResultFlags::WITH_AVAILABILITY,
            )
            .expect("Failed to get results");

        let start_t = results[0] as f64;
        let start_available = results[1];

        let end_t = results[2] as f64;
        let end_available = results[3];

        if end_available == 0 || start_available == 0 {
            return None;
        }

        let timestamp_period = vk.physical_device.properties().timestamp_period as f64;

        Some((end_t - start_t) * timestamp_period / 1000000.0)
    }

    /// Timestamps range must be at least two items.
    /// This will assume the start is the first item, the end is the second item.
    /// Will ignore any other timestamps outside of the first two items.
    pub fn get_delta_ms_wait(&self, vk: &VulkanInstance, timestamps: Range<u32>) -> f64 {
        let Some(Inner { query_pool }) = self.0.as_ref() else {
            return f64::NAN;
        };

        let mut results: Vec<u64> = vec![0; query_pool.query_count() as usize];
        query_pool
            .get_results(timestamps, &mut results, QueryResultFlags::WAIT)
            .expect("Failed to get results");

        let start_t = results[0] as f64;
        let end_t = results[1] as f64;
        let timestamp_period = vk.physical_device.properties().timestamp_period as f64;

        (end_t - start_t) * timestamp_period / 1000000.0
    }
}
