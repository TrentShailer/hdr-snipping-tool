#version 450

layout(location = 0) in vec2 position;
layout(location = 1) in vec2 uv;
layout(location = 2) in uint color;

layout(location = 0) out vec2 f_uv;
layout(location = 1) out vec4 f_color;


void main() {
    f_uv = uv;
    f_color = unpackUnorm4x8(color);

    gl_Position = vec4(position, 0.0, 1.0);
}
