#version 450

layout(location = 0) in vec2 position;
layout(location = 1) in vec2 uv;

// per instance data
layout(location = 2) in vec2 position_offset; // in px units
layout(location = 3) in vec2 size; // in px units
layout(location = 4) in vec2 bitmap_size; // in px units
layout(location = 5) in vec2 uv_offset; // scaled

layout(push_constant) uniform PushConstants {
    vec2 text_position; // scaled to be in vulkan coordinates
	vec2 window_size; // in px
	float font_size;
};

layout(location = 0) out vec2 out_uv;

void main() {
	vec2 bitmap_scale = bitmap_size / font_size;
	out_uv = uv * bitmap_scale + uv_offset;

	vec2 scale = size / window_size;
	vec2 offset = (position_offset / window_size) * 2.0;
	gl_Position = vec4(position * scale + vec2(scale) + offset + text_position, 0.0, 1.0);
}
