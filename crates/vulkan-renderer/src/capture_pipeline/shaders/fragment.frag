#version 450

layout(location = 0) in vec2 in_uv;

layout(location = 0) out vec4 out_color;

layout(set = 0, binding = 0) uniform sampler capture_sampler;
layout(set = 0, binding = 1) uniform texture2D capture;

void main() {
    out_color = texture(sampler2D(capture, capture_sampler), in_uv);
}
