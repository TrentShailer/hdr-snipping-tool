#version 450

layout(location = 0) in vec2 position;
layout(location = 1) in vec4 color;
layout(location = 2) in uint flags;
// locked_bit, top bit, left bit

layout(location = 0) out vec4 out_color;

layout(push_constant) uniform PushConstants {
	vec2 base_position;
	vec2 base_size;

    vec2 target_position;
	vec2 target_size;
};

void main() {
	bool locked = (flags & 1 << 0) >> 0 == 1;

    out_color = color;

	if (locked) {
    	gl_Position = vec4(position, 0.0, 1.0);
	}
	else {
		// calcuate the scale that should be applied to the rect
		vec2 scale = target_size / base_size;
		vec2 scaled_position = position * scale;

		// Work out the position offset to move the rect to the target position
		vec2 position_offset = target_position - base_position;

		vec2 out_position = scaled_position + position_offset;

		gl_Position = vec4(out_position, 0.0, 1.0);
	}

}
