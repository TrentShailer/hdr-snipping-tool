// slangc -g2 -emit-spirv-directly -target spirv -profile spirv_1_4  crates/vulkan/src/_shaders/histogram.slang -o crates/vulkan/src/_shaders/spv/histogram.spv

import utilities;

ParameterBlock<Descriptor> descriptor;
struct Descriptor
{
    [vk_image_format("rgba16f")]
    RWTexture2D<float4> image;
    RWByteAddressBuffer histogram;
}

[vk::push_constant]
ConstantBuffer<PushConstants> push_constants;
struct PushConstants
{
    double bin_width;
}

static const uint2 OUT_OF_BOUNDS_INDEX = { 0, 0 };

[shader("compute")]
[numthreads(16, 16, 1)]
void main(
    uint3 global_id: SV_DispatchThreadID,
    uint3 group_id: SV_GroupID,
    uint group_index: SV_GroupIndex)
{
    uint width, height;
    descriptor.image.GetDimensions(width, height);

    uint2 index = uint2(global_id.x, global_id.y);
    let in_bounds = index.x < width && index.y < height;

    index = in_bounds ? index : OUT_OF_BOUNDS_INDEX;
    let value = in_bounds ? 1 : 0;

    float4 colour = descriptor.image[index.xy];

    let r_bin_index = uint(floor(double(colour.r) / push_constants.bin_width));
    let g_bin_index = uint(floor(double(colour.g) / push_constants.bin_width));
    let b_bin_index = uint(floor(double(colour.b) / push_constants.bin_width));

    descriptor.histogram.InterlockedAdd(r_bin_index * 4, value);
    descriptor.histogram.InterlockedAdd(g_bin_index * 4, value);
    descriptor.histogram.InterlockedAdd(b_bin_index * 4, value);
}
