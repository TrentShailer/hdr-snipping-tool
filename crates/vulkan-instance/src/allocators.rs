use std::sync::Arc;

use vulkano::{
    command_buffer::allocator::StandardCommandBufferAllocator,
    descriptor_set::allocator::StandardDescriptorSetAllocator,
    device::Device,
    memory::{
        allocator::{GenericMemoryAllocatorCreateInfo, StandardMemoryAllocator},
        MemoryProperties,
    },
};

pub struct Allocators {
    pub memory: Arc<StandardMemoryAllocator>,
    pub command: Arc<StandardCommandBufferAllocator>,
    pub descriptor: Arc<StandardDescriptorSetAllocator>,
}

impl Allocators {
    pub fn new(device: Arc<Device>) -> Self {
        let MemoryProperties {
            memory_types,
            memory_heaps: _,
            ..
        } = device.physical_device().memory_properties();

        // 1MiB block size means that most allocations for captures/images
        // will be dedicated allocations, this is slower but results in
        // overall reduced memory consumption at idle
        let block_sizes = vec![1024 * 1024; memory_types.len()];

        let memory = Arc::new(StandardMemoryAllocator::new(
            device.clone(),
            GenericMemoryAllocatorCreateInfo {
                block_sizes: &block_sizes,
                ..Default::default()
            },
        ));

        let descriptor = Arc::new(StandardDescriptorSetAllocator::new(
            device.clone(),
            Default::default(),
        ));

        let command = Arc::new(StandardCommandBufferAllocator::new(
            device.clone(),
            Default::default(),
        ));

        Self {
            memory,
            command,
            descriptor,
        }
    }
}
