#version 450

layout(location = 0) in vec2 position;
layout(location = 1) in vec4 color;

layout(location = 0) out vec4 out_color;

layout(push_constant) uniform PushConstants {
    vec2 rect_position;
	vec2 rect_scale;
};

void main() {
    out_color = color;

	vec2 scaled = position * rect_scale;
	gl_Position = vec4(scaled + vec2(rect_scale) + rect_position, 0.0, 1.0);
}
