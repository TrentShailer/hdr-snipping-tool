#version 450

layout(location = 0) in vec2 position;
layout(location = 1) in vec4 color;
layout(location = 2) in uint flags;
// outer bit, top bit, left bit

layout(location = 0) out vec4 out_color;

layout(push_constant) uniform PushConstants {
    vec2 border_position;
    vec2 border_scale;
	vec2 line_size;
};

void main() {
	bool outer	= (flags & 1 << 2) >> 2 == 1;
	bool top 	= (flags & 1 << 1) >> 1 == 1;
	bool left 	= (flags & 1 << 0) >> 0 == 1;

    out_color = color;

	vec2 scaled = position * border_scale;
	vec2 out_position = scaled + vec2(border_scale) + border_position;

	// offset border_ltrb position based on position and line size
	if ((left && outer) || (!left && !outer)) {
		// shift points left
		out_position.x -= line_size.x;
	}
	else {
		// shift points right
		out_position.x += line_size.x;
	}

	if ((top && outer) || (!top && !outer)) {
		// shift points up
		out_position.y -= line_size.y;
	}
	else {
		// shift poitns down
		out_position.y += line_size.y;
	}


	gl_Position = vec4(out_position, 0.0, 1.0);
}
