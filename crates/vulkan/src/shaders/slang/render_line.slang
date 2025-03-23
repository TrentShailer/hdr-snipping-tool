[vk::push_constant]
ConstantBuffer<Line> line;
struct Line
{
    float2 start;  // Offset 0
    float2 end;    // Offset 8
    float4 colour; // Offset 16
}

struct VSInput
{
    uint index;
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

    if (input.index == 0)
    {
        output.position = float4(line.start, 0.0, 1.0);
    }
    else
    {
        output.position = float4(line.end, 0.0, 1.0);
    }

    output.colour = line.colour;

    return output;
}

[shader("fragment")]
float4 fragmentMain(FSInput input)
    : SV_Target
{
    return input.colour;
}
