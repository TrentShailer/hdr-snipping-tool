import utilities;

ParameterBlock<Descriptor> descriptor;
struct Descriptor
{
    [vk_image_format("rgba16f")]
    RWTexture2D<float4> input;

    [vk_image_format("rgba8")]
    RWTexture2D<float4> output;
}

[vk::push_constant]
ConstantBuffer<PushConstants> push_constants;
struct PushConstants
{
    float whitepoint;
}

static const uint2 OUT_OF_BOUNDS_INDEX = { 0, 0 };

[shader("compute")]
[numthreads(64, 4, 1)]
void main(uint3 global_id: SV_DispatchThreadID)
{
    uint width, height;
    descriptor.input.GetDimensions(width, height);
    uint2 extent = { width, height };

    uint2 index = global_id.xy;
    index = index.x < index.x || index.y < index.y ? index : OUT_OF_BOUNDS_INDEX;

    let linear_rgb = descriptor.input[global_id.xy];

    float4 s_rgb = {
        tonemap(linear_rgb.r, push_constants.whitepoint),
        tonemap(linear_rgb.g, push_constants.whitepoint),
        tonemap(linear_rgb.b, push_constants.whitepoint),
        linear_rgb.a
    };

    descriptor.output[global_id.xy] = s_rgb;
}
