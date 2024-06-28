#version 450

layout(location = 0) in vec2 position;
layout(location = 1) in vec4 color;
layout(location = 2) in uint flags;
// outer bit, top bit, left bit

layout(location = 0) out vec4 out_color;

layout(push_constant) uniform PushConstants {
    vec2 base_position;
    vec2 base_size;

    vec2 target_position;
    vec2 target_size;
	vec2 line_size;
};

void main() {
	bool outer	= (flags & 1 << 2) >> 2 == 1;
	bool top 	= (flags & 1 << 1) >> 1 == 1;
	bool left 	= (flags & 1 << 0) >> 0 == 1;

    out_color = color;

	// calcuate the scale that should be applied to the rect
	vec2 scale = target_size / base_size;
	vec2 scaled_position = position * scale;

	// Work out the position offset to move the rect to the target position
	vec2 position_offset = target_position - base_position;

	vec2 out_position = scaled_position + position_offset;

	// offset border_ltrb position based on position and line size
	if ((left && outer) || (!left && !outer)) {
		// shift points left
		out_position.x -= line_size.x / 2.0;
	}
	else {
		// shift points right
		out_position.x += line_size.x / 2.0;
	}

	if ((top && outer) || (!top && !outer)) {
		// shift points up
		out_position.y -= line_size.y / 2.0;
	}
	else {
		// shift poitns down
		out_position.y += line_size.y / 2.0;
	}


	gl_Position = vec4(out_position, 0.0, 1.0);
}
