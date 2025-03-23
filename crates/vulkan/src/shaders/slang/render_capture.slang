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

struct PushConstants
{
    float whitepoint;
    float max_brightness;
    uint present_srgb;
}

[vk::push_constant]
ConstantBuffer<PushConstants> push_constants;

[vk_binding(0, 0)]
ParameterBlock<Descriptor> descriptor;
struct Descriptor
{
    [format("rgba16f")]
    Sampler2D<float4> sampler;
}

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
    let max_brightness = push_constants.max_brightness;
    let whitepoint = push_constants.whitepoint;

    let colour = descriptor.sampler.Sample(input.uv);

    if (push_constants.present_srgb == 1)
    {
        return {
            tonemap(colour.r, push_constants.whitepoint),
            tonemap(colour.g, push_constants.whitepoint),
            tonemap(colour.b, push_constants.whitepoint),
            colour.a
        };
    }
    else
    {
        return {
            (clamp(colour.r, 0.0, whitepoint) / whitepoint) * max_brightness,
            (clamp(colour.g, 0.0, whitepoint) / whitepoint) * max_brightness,
            (clamp(colour.b, 0.0, whitepoint) / whitepoint) * max_brightness,
            colour.a
        };
    }
}
