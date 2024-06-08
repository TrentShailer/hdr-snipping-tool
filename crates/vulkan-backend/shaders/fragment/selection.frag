#version 450

layout(location = 0) in vec2 in_uv;
layout(location = 0) out vec4 out_color;

layout(push_constant) uniform PushConstants {
	vec4 selection;
	vec2 limits;
};

void main() {    
    float l = selection[0];
    float t = selection[1];
    float r = selection[2];
    float b = selection[3];

    if (in_uv.x < l || in_uv.x > r || in_uv.y < t || in_uv.y > b)  {
        out_color = vec4(0.0, 0.0, 0.0, 0.5);
    }
    else {
        out_color = vec4(0.0, 0.0, 0.0, 0.0);
    }
}