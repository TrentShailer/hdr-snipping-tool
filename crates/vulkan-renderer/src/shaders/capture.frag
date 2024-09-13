#version 450

layout(location = 0) in vec2 in_uv;

layout(location = 0) out vec4 out_color;

layout(set = 0, binding = 0) uniform sampler capture_sampler;
layout(set = 1, binding = 0) uniform texture2D capture;

layout(push_constant) uniform PushConstants {
	 float whitepoint;
	 uint flags;
};

const uint ENCODE_AS_SRGB = 1;

// Performs gamma correction to tonemap linear RGB to sRGB
// From https://en.wikipedia.org/wiki/SRGB#Transformation
float scRGB_to_sRGB(float value) {
	float gamma_decode = 1.0 / 2.4;

	float clamped_value = clamp(value, 0.0, whitepoint);
	float scaled_value = clamped_value / whitepoint;

	if (scaled_value <= 0.0031308) {
		return clamp(12.92 * scaled_value, 0.0, 1.0);
	}
	else {
		return clamp(1.055 * pow(scaled_value, gamma_decode) - 0.055, 0.0, 1.0);
	}
}

void main() {
	bool encode_as_srgb = (flags & ENCODE_AS_SRGB) == ENCODE_AS_SRGB;

	vec4 in_color = texture(sampler2D(capture, capture_sampler), in_uv);


	if (encode_as_srgb) {
		vec4 srgb = vec4(
			scRGB_to_sRGB(in_color.r),
			scRGB_to_sRGB(in_color.g),
			scRGB_to_sRGB(in_color.b),
			in_color.a
		);

		out_color = srgb;
	}
	else {
		vec4 clamped_color = vec4(
			clamp(in_color.r, 0.0, whitepoint),
			clamp(in_color.g, 0.0, whitepoint),
			clamp(in_color.b, 0.0, whitepoint),
			in_color.a
		);

		out_color = clamped_color;
	}

}
