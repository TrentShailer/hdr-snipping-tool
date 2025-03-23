import utilities;

ParameterBlock<Descriptor> descriptor;
struct Descriptor
{
    [vk_image_format("rgba16f")]
    RWTexture2D<float4> image;
    RWByteAddressBuffer maximum;
}

static const uint2 OUT_OF_BOUNDS_INDEX = { 0, 0 };

[shader("compute")]
[numthreads(64, 4, 1)]
void main(
    uint3 global_id: SV_DispatchThreadID,
    uint3 group_id: SV_GroupID,
    uint group_index: SV_GroupIndex)
{
    // Fetch the image dimentions for bounds checking.
    uint width, height;
    descriptor.image.GetDimensions(width, height);

    // Bounds check
    let in_bounds = global_id.x < width && global_id.y < height;

    // Determine the fetch index
    uint2 index = uint2(global_id.x, global_id.y);
    index = in_bounds ? index : OUT_OF_BOUNDS_INDEX;

    // Load the colour
    float4 colour = descriptor.image[index.xy];

    // Find the maximum from the R, G, and B values
    float maximum = max3(colour.r, colour.g, colour.b);
    maximum = in_bounds ? maximum : 0.0;

    // Use atomic operation to write the result if it is larger.
    // Atomic max on a float does not exist
    // However, bitcasting to a uint is fine as long as there is one value in the input that is
    // non-negative.
    let uint_maximum = asuint(max(maximum, 0.0));
    descriptor.maximum.InterlockedMax(0, uint_maximum);
}
