#version 450

layout(location = 0) in vec2 position;
layout(location = 1) in vec4 color;
layout(location = 2) in uint flags;
// locked_bit, top bit, left bit

layout(location = 0) out vec4 out_color;

layout(push_constant) uniform PushConstants {
    vec2 selection_position;
	vec2 selection_scale;
};

void main() {
	bool locked = (flags & 1 << 0) >> 0 == 1;

    out_color = color;

	if (locked) {
    	gl_Position = vec4(position, 0.0, 1.0);
	}
	else {
		vec2 scaled = position * selection_scale;
		vec2 out_position = scaled + vec2(selection_scale) + selection_position;

		gl_Position = vec4(out_position, 0.0, 1.0);
	}

}
