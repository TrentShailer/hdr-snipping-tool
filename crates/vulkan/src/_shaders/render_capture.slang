// slangc -g2 -emit-spirv-directly -target spirv -profile spirv_1_4  crates/vulkan/src/_shaders/render_capture.slang -o crates/vulkan/src/_shaders/spv/render_capture.spv

import utilities;

struct VSInput
{
    float2 position;
    float2 uv;
};

struct FSInput
{
    float4 position : SV_Position;
    float2 uv : UV;
};

[vk::push_constant]
ConstantBuffer<float> whitepoint;

[vk_binding(0, 0)]
ParameterBlock<Descriptor> descriptor;
struct Descriptor
{
    [format("rgba16f")]
    Sampler2D<float4> sampler;
}

[SpecializationConstant]
const uint present_srgb = 0;

[shader("vertex")]
FSInput vertexMain(VSInput input)
{
    FSInput output;

    output.position = float4(input.position, 0.0, 1.0);
    output.uv = input.uv;

    return output;
}

[shader("fragment")]
float4 fragmentMain(FSInput input)
    : SV_Target
{
    let colour = descriptor.sampler.Sample(input.uv);

    if (present_srgb == 1)
    {
        return {
            tonemap(colour.r, whitepoint),
            tonemap(colour.g, whitepoint),
            tonemap(colour.b, whitepoint),
            colour.a
        };
    }
    else
    {
        return {
            clamp(colour.r, 0.0, whitepoint),
            clamp(colour.g, 0.0, whitepoint),
            clamp(colour.b, 0.0, whitepoint),
            colour.a
        };
    }
}
