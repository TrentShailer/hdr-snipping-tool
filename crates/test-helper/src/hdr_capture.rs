use ash::{
    util::Align,
    vk::{
        AccessFlags2, BufferCreateInfo, BufferImageCopy2, BufferUsageFlags, CopyBufferToImageInfo2,
        DependencyInfo, DeviceMemory, Extent2D, Format, Image, ImageAspectFlags, ImageCreateInfo,
        ImageLayout, ImageMemoryBarrier2, ImageSubresourceLayers, ImageSubresourceRange,
        ImageTiling, ImageType, ImageUsageFlags, ImageView, ImageViewCreateInfo, ImageViewType,
        MemoryAllocateInfo, MemoryMapFlags, MemoryPropertyFlags, Offset3D, PipelineStageFlags2,
        SampleCountFlags, SharingMode, QUEUE_FAMILY_IGNORED,
    },
};
use half::f16;
use vulkan_instance::{CommandBufferUsage, VulkanInstance};

pub const MAXIMUM: f16 = f16::from_f32_const(12.5);

pub fn get_hdr_image(vk: &VulkanInstance) -> (Image, DeviceMemory, ImageView, [u32; 2]) {
    let size = [1920u32, 1080u32];
    let data_len = size[0] as u64 * size[1] as u64;
    let data_bytes_len = data_len * 4 * 2;

    let data: Box<[f16]> = (0..data_len)
        .flat_map(|index| {
            let r = f16::from_f32(index as f32 / (data_len - 1) as f32) * MAXIMUM;
            let g = f16::from_f32(index as f32 / (data_len - 1) as f32) * MAXIMUM;
            let b = f16::from_f32(index as f32 / (data_len - 1) as f32) * MAXIMUM;
            let a = f16::from_f32(1.0);

            [r, g, b, a]
        })
        .collect();

    let real_max = data.iter().max_by(|a, b| a.total_cmp(b)).unwrap();
    dbg!(real_max);

    let image = unsafe {
        let image_extent = Extent2D {
            width: size[0],
            height: size[1],
        };

        let image_create_info = ImageCreateInfo {
            image_type: ImageType::TYPE_2D,
            format: Format::R16G16B16A16_SFLOAT,
            extent: image_extent.into(),
            mip_levels: 1,
            array_layers: 1,
            samples: SampleCountFlags::TYPE_1,
            tiling: ImageTiling::OPTIMAL,
            usage: ImageUsageFlags::TRANSFER_SRC
                | ImageUsageFlags::TRANSFER_DST
                | ImageUsageFlags::STORAGE,
            sharing_mode: SharingMode::EXCLUSIVE,
            initial_layout: ImageLayout::UNDEFINED,
            ..Default::default()
        };

        vk.device.create_image(&image_create_info, None).unwrap()
    };

    let image_memory = unsafe {
        let memory_requirement = vk.device.get_image_memory_requirements(image);

        let memory_index = vk
            .find_memorytype_index(&memory_requirement, MemoryPropertyFlags::DEVICE_LOCAL)
            .unwrap();

        let allocate_info = MemoryAllocateInfo {
            allocation_size: memory_requirement.size,
            memory_type_index: memory_index,
            ..Default::default()
        };

        let device_memory = vk.device.allocate_memory(&allocate_info, None).unwrap();

        device_memory
    };

    unsafe {
        vk.device.bind_image_memory(image, image_memory, 0).unwrap();
    };

    // create and write to staging buffer
    let (staging_buffer, staging_buffer_memory) = unsafe {
        let buffer_create_info = BufferCreateInfo::default()
            .size(data_bytes_len)
            .usage(BufferUsageFlags::TRANSFER_SRC)
            .sharing_mode(SharingMode::EXCLUSIVE);

        let staging_buffer = vk.device.create_buffer(&buffer_create_info, None).unwrap();

        let staging_buffer_memory_requirements =
            vk.device.get_buffer_memory_requirements(staging_buffer);

        let staging_buffer_memory_index = vk
            .find_memorytype_index(
                &staging_buffer_memory_requirements,
                MemoryPropertyFlags::HOST_VISIBLE | MemoryPropertyFlags::HOST_COHERENT,
            )
            .unwrap();

        let staging_buffer_allocate_info = MemoryAllocateInfo::default()
            .allocation_size(staging_buffer_memory_requirements.size)
            .memory_type_index(staging_buffer_memory_index);

        let staging_buffer_memory = vk
            .device
            .allocate_memory(&staging_buffer_allocate_info, None)
            .unwrap();

        let staging_ptr = vk
            .device
            .map_memory(
                staging_buffer_memory,
                0,
                data_bytes_len,
                MemoryMapFlags::empty(),
            )
            .unwrap();

        let mut staging_slice = Align::new(
            staging_ptr,
            std::mem::align_of::<f16>() as u64,
            data_bytes_len,
        );
        let data_slice: &[f16] = &data;
        staging_slice.copy_from_slice(data_slice);
        vk.device.unmap_memory(staging_buffer_memory);

        vk.device
            .bind_buffer_memory(staging_buffer, staging_buffer_memory, 0)
            .unwrap();

        (staging_buffer, staging_buffer_memory)
    };

    // copy from staging to gpu
    vk.record_submit_command_buffer(
        CommandBufferUsage::Setup,
        &[],
        &[],
        |device, command_buffer| {
            let memory_barriers = [ImageMemoryBarrier2 {
                src_stage_mask: PipelineStageFlags2::NONE,
                src_access_mask: AccessFlags2::NONE,
                dst_stage_mask: PipelineStageFlags2::TRANSFER,
                dst_access_mask: AccessFlags2::MEMORY_WRITE,
                old_layout: ImageLayout::UNDEFINED,
                new_layout: ImageLayout::TRANSFER_DST_OPTIMAL,
                src_queue_family_index: QUEUE_FAMILY_IGNORED,
                dst_queue_family_index: QUEUE_FAMILY_IGNORED,
                image,
                subresource_range: ImageSubresourceRange {
                    aspect_mask: ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                },
                ..Default::default()
            }];

            let dependency_info = DependencyInfo::default().image_memory_barriers(&memory_barriers);

            unsafe { device.cmd_pipeline_barrier2(command_buffer, &dependency_info) }

            let extent = Extent2D {
                width: size[0],
                height: size[1],
            };
            let image_subresource = ImageSubresourceLayers {
                aspect_mask: ImageAspectFlags::COLOR,
                mip_level: 0,
                base_array_layer: 0,
                layer_count: 1,
            };

            let copy_regions = [BufferImageCopy2 {
                buffer_offset: 0,
                buffer_row_length: 0,
                buffer_image_height: 0,
                image_subresource,
                image_offset: Offset3D::default(),
                image_extent: extent.into(),
                ..Default::default()
            }];

            let image_copy_info = CopyBufferToImageInfo2::default()
                .src_buffer(staging_buffer)
                .dst_image(image)
                .dst_image_layout(ImageLayout::TRANSFER_DST_OPTIMAL)
                .regions(&copy_regions);

            unsafe { device.cmd_copy_buffer_to_image2(command_buffer, &image_copy_info) }

            let memory_barriers = [ImageMemoryBarrier2 {
                src_stage_mask: PipelineStageFlags2::TRANSFER,
                src_access_mask: AccessFlags2::MEMORY_WRITE,
                dst_stage_mask: PipelineStageFlags2::BOTTOM_OF_PIPE,
                dst_access_mask: AccessFlags2::NONE,
                old_layout: ImageLayout::TRANSFER_DST_OPTIMAL,
                new_layout: ImageLayout::GENERAL,
                src_queue_family_index: QUEUE_FAMILY_IGNORED,
                dst_queue_family_index: QUEUE_FAMILY_IGNORED,
                image,
                subresource_range: ImageSubresourceRange {
                    aspect_mask: ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                },
                ..Default::default()
            }];

            let dependency_info = DependencyInfo::default().image_memory_barriers(&memory_barriers);

            unsafe { device.cmd_pipeline_barrier2(command_buffer, &dependency_info) }
        },
    )
    .unwrap();

    unsafe {
        vk.device.wait_for_fences(
            &[*vk.fences.get(&CommandBufferUsage::Setup).unwrap()],
            true,
            u64::MAX,
        )
    }
    .unwrap();

    // clean up
    unsafe {
        vk.device.destroy_buffer(staging_buffer, None);
        vk.device.free_memory(staging_buffer_memory, None);
    }

    let image_view = unsafe {
        let image_view_create_info = ImageViewCreateInfo {
            image,
            view_type: ImageViewType::TYPE_2D,
            format: Format::R16G16B16A16_SFLOAT,
            subresource_range: ImageSubresourceRange {
                aspect_mask: ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            },
            ..Default::default()
        };

        vk.device
            .create_image_view(&image_view_create_info, None)
            .unwrap()
    };

    (image, image_memory, image_view, size)
}
