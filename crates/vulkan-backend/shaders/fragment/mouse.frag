#version 450

layout(location = 0) in vec2 in_uv;
layout(location = 0) out vec4 out_color;

layout(push_constant) uniform PushConstants {
	vec2 mouse_position;
    vec2 limits;
};

bool is_close(float source, float target, float limit) {
    float diff = abs(target - source);
    return diff < limit;
}

void main() {
    if (
        is_close(in_uv.x, mouse_position.x, limits.x) ||
        is_close(in_uv.y, mouse_position.y, limits.y)
    ) {
        out_color = vec4(1.0, 1.0, 1.0, 1.0);
    }
    else {
        out_color = vec4(0.0, 0.0, 0.0, 1.0);
    }
}