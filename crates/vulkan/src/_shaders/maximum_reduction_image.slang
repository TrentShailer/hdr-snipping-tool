// slangc -g2 -emit-spirv-directly -target spirv -profile spirv_1_4 crates/vulkan/src/_shaders/maximum_reduction_image.slang -o crates/vulkan/src/_shaders/spv/maximum_reduction_image.spv

import utilities;

ParameterBlock<Descriptor> descriptor;
struct Descriptor
{
    [vk_image_format("rgba16f")]
    RWTexture2D<float4> image;
    RWStructuredBuffer<half> buffer;
}

static const uint2 OUT_OF_BOUNDS_INDEX = { 0, 0 };

[shader("compute")]
[numthreads(16, 16, 1)]
void main(
    uint3 global_id: SV_DispatchThreadID,
    uint3 group_id: SV_GroupID,
    uint group_index: SV_GroupIndex)
{
    let lane_count = WaveGetLaneCount();
    let lane_index = WaveGetLaneIndex();
    let workgroup_count = GetWorkgroupCount();
    let waves_per_dispatch = (16 * 16) / lane_count;

    uint width, height;
    descriptor.image.GetDimensions(width, height);
    uint2 extent = { width, height };

    uint2 start_id = { global_id.x * 2, global_id.y * 2 };

    uint2 index0 = { start_id.x + 0, start_id.y + 0 };
    uint2 index1 = { start_id.x + 0, start_id.y + 1 };
    uint2 index2 = { start_id.x + 1, start_id.y + 1 };
    uint2 index3 = { start_id.x + 1, start_id.y + 0 };

    index0 = index0.x < extent.x || index0.y < extent.y ? index0 : OUT_OF_BOUNDS_INDEX;
    index1 = index1.x < extent.x || index1.y < extent.y ? index1 : OUT_OF_BOUNDS_INDEX;
    index2 = index2.x < extent.x || index2.y < extent.y ? index2 : OUT_OF_BOUNDS_INDEX;
    index3 = index3.x < extent.x || index3.y < extent.y ? index3 : OUT_OF_BOUNDS_INDEX;

    float4 colour0 = descriptor.image[index0.xy];
    float4 colour1 = descriptor.image[index1.xy];
    float4 colour2 = descriptor.image[index2.xy];
    float4 colour3 = descriptor.image[index3.xy];

    let local_max0 = max(max4(colour0), max4(colour1));
    let local_max1 = max(max4(colour2), max4(colour3));

    let local_max = max(local_max0, local_max1);

    let wave_maximum = WaveActiveMax(local_max);

    let local_wave_index = group_index / lane_count;
    let dispatch_index = group_id.y * workgroup_count.x + group_id.x;
    let wave_index = dispatch_index * waves_per_dispatch + local_wave_index;

    if (WaveIsFirstLane())
    {
        descriptor.buffer[wave_index] = half(wave_maximum);
    }
}
