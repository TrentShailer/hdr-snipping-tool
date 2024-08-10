#version 450

layout(location = 0) in vec2 in_uv;

layout(location = 0) out vec4 out_color;

layout(set = 0, binding = 0) uniform sampler capture_sampler;
layout(set = 0, binding = 1) uniform texture2D capture;

layout(push_constant) uniform PushConstants {
	 float whitepoint;
};

void main() {
	vec4 in_color = texture(sampler2D(capture, capture_sampler), in_uv);

	vec4 clamped_color = vec4(
		clamp(in_color.r, 0.0, whitepoint),
		clamp(in_color.g, 0.0, whitepoint),
		clamp(in_color.b, 0.0, whitepoint),
		in_color.a
	);

    out_color = clamped_color;
}
