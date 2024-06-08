#version 450

layout(location = 0) in vec2 in_uv;
layout(location = 0) out vec4 out_color;

layout(push_constant) uniform PushConstants {
	vec4 selection;
	vec2 limits;
};

bool is_close(float source, float target, float limit) {
    float diff = abs(target - source);
    return diff < limit;
}

bool is_border(vec2 pos, float l, float t, float r, float b, vec2 limits) {
	return	(is_close(pos.x, l, limits.x) && pos.y > t && pos.y < b) ||
			(is_close(pos.y, t, limits.y) && pos.x > l && pos.x < r) ||
			(is_close(pos.x, r, limits.x) && pos.y > t && pos.y < b) ||
			(is_close(pos.y, b, limits.y) && pos.x > l && pos.x < r);
}

void main() {    
    float l = selection[0];
    float t = selection[1];
    float r = selection[2];
    float b = selection[3];

	if (is_border(in_uv, l, t, r, b, limits))  {
		out_color = vec4(0.8, 0.8, 0.8, 1.0);
	}
    else {
        out_color = vec4(0.0, 0.0, 0.0, 0.0);
    }
}