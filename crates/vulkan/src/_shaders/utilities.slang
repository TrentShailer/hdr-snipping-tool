module utilities;

// Performs gamma correction to tonemap linear RGB to sRGB
// From https://en.wikipedia.org/wiki/SRGB#Transformation
public float tonemap(float value, float whitepoint)
{
    let gamma_decode = 1.0 / 2.4;

    let clamped_value = clamp(value, 0.0, whitepoint);
    let scaled_value = clamped_value / whitepoint;

    if (scaled_value <= 0.0031308)
    {
        return clamp(12.92 * scaled_value, 0.0, 1.0);
    }
    else
    {
        return clamp(1.055 * pow(scaled_value, gamma_decode) - 0.055, 0.0, 1.0);
    }
}

public int3 GetWorkgroupCount()
{
    __target_switch
    {
    case glsl:
        __intrinsic_asm "gl_NumWorkGroups";
    case spirv:
        return spirv_asm
        {
                result:$$int3 = OpLoad builtin(NumWorkgroups:int3);
           };
    }
}

public float max4(float4 value)
{
    return max(max3(value.x, value.y, value.z), value.w);
}
