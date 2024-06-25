#version 450

layout(location = 0) in vec2 in_uv;

layout(location = 0) out vec4 out_color;

layout(set = 0, binding = 0) uniform sampler atlas_sampler;
layout(set = 0, binding = 1) uniform texture2D atlas;

void main() {
	float alpha = texture(sampler2D(atlas, atlas_sampler), in_uv).r;
    out_color = vec4(1.0, 1.0, 1.0, alpha);
}
