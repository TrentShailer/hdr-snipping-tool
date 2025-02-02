// slangc -g2 -emit-spirv-directly -target spirv -profile spirv_1_4  crates/vulkan/src/_shaders/render_selection.slang -o crates/vulkan/src/_shaders/spv/render_selection.spv

[vk::push_constant]
ConstantBuffer<Selection> selection;
struct Selection
{
    float2 start; // Offset 0
    float2 end;   // Offset 8
}

enum Placement
{
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight
}

struct VSInput
{
    float2 position; // Offset 0
    float4 colour;   // Offset 8
    uint placement;  // Offset 24
    uint movable;    // Offset 28
}

struct FSInput
{
    float4 position : SV_Position;
    float4 colour : COLOUR;
}

[shader("vertex")]
FSInput vertexMain(VSInput input)
{
    FSInput output;
    output.colour = input.colour;

    if (input.movable == 0)
    {
        output.position = float4(input.position, 0.0, 1.0);
        return output;
    }

    let left = min(selection.start.x, selection.end.x);
    let right = max(selection.start.x, selection.end.x);
    let top = min(selection.start.y, selection.end.y);
    let bottom = max(selection.start.y, selection.end.y);

    let placement = Placement(input.placement);
    switch (placement)
    {
    case Placement.TopLeft:
        output.position = float4(left, top, 0.0, 1.0);
        break;
    case Placement.TopRight:
        output.position = float4(right, top, 0.0, 1.0);
        break;
    case Placement.BottomLeft:
        output.position = float4(left, bottom, 0.0, 1.0);
        break;
    case Placement.BottomRight:
        output.position = float4(right, bottom, 0.0, 1.0);
        break;
    default:
        output.position = float4(0.0, 0.0, 0.0, 1.0);
    }

    return output;
}

[shader("fragment")]
float4 fragmentMain(FSInput input)
    : SV_Target
{
    return input.colour;
}
