use std::sync::Arc;

use vulkan_instance::VulkanInstance;
use vulkano::{
    command_buffer::{
        allocator::StandardCommandBufferAllocator, AutoCommandBufferBuilder,
        PrimaryAutoCommandBuffer,
    },
    query::{QueryPool, QueryPoolCreateInfo, QueryResultFlags},
    sync,
};

use crate::Tonemapper;

pub fn maybe_create_timestamp_pool(vk: &VulkanInstance) -> Option<Arc<QueryPool>> {
    if std::env::var("hdr-snipping-tool-debug").is_err()
        || !vk.supported_optional_features.host_query_reset
    {
        return None;
    }

    let mut query_info = QueryPoolCreateInfo::query_type(vulkano::query::QueryType::Timestamp);
    query_info.query_count = 2;

    let result = QueryPool::new(vk.device.clone(), query_info);

    if let Err(e) = result.as_ref() {
        log::error!("Failed to create query pool:\n{0:?}", e);
    }

    result.ok()
}

pub fn maybe_log_tonemap_time(vk: &VulkanInstance, tonemapper: &Tonemapper) {
    let pool = match tonemapper.timestamp_pool.as_ref() {
        Some(v) => v,
        None => return,
    };

    let mut results: Vec<u64> = vec![0; pool.query_count() as usize];

    pool.get_results(0..2, &mut results, QueryResultFlags::WAIT)
        .unwrap();

    let start_t = results[0] as f64;
    let end_t = results[1] as f64;
    let timestamp_period = vk.physical_device.properties().timestamp_period as f64;

    let delta = (end_t - start_t) * timestamp_period / 1000000.0;

    log::debug!("Tonemapped in {}ms", delta);
}

pub fn maybe_reset(
    tonemapper: &Tonemapper,
    builder: &mut AutoCommandBufferBuilder<
        PrimaryAutoCommandBuffer<Arc<StandardCommandBufferAllocator>>,
        Arc<StandardCommandBufferAllocator>,
    >,
) {
    let pool = match tonemapper.timestamp_pool.as_ref() {
        Some(v) => v,
        None => return,
    };

    unsafe {
        builder.reset_query_pool(pool.clone(), 0..2).unwrap();
    }
}

pub fn maybe_record_timestamp(
    tonemapper: &Tonemapper,
    builder: &mut AutoCommandBufferBuilder<
        PrimaryAutoCommandBuffer<Arc<StandardCommandBufferAllocator>>,
        Arc<StandardCommandBufferAllocator>,
    >,
    index: u32,
    stage: sync::PipelineStage,
) {
    let pool = match tonemapper.timestamp_pool.as_ref() {
        Some(v) => v,
        None => return,
    };

    unsafe {
        builder.write_timestamp(pool.clone(), index, stage).unwrap();
    };
}
