#version 450

layout(location = 0) in vec2 position;
layout(location = 1) in vec4 color;
layout(location = 2) in uint flags;
// outer bit, top bit, left bit

layout(location = 0) out vec4 out_color;

layout(push_constant) uniform PushConstants {
    vec4 border_points; // ltrb, scaled to be in vulkan coordinates
	vec2 line_size; // scaled in vulkan coordinates
};

void main() {
	bool outer	= (flags & 1 << 2) >> 2 == 1;
	bool top 	= (flags & 1 << 1) >> 1 == 1;
	bool left 	= (flags & 1 << 0) >> 0 == 1;

    out_color = color;

	// Get border_points position for this vertex
	float x = 0.0f;
	float y = 0.0f;

	if (top) {
		y = border_points[1];
	}
	else {
		y = border_points[3];
	}

	if (left) {
		x = border_points[0];
	}
	else {
		x = border_points[2];
	}

	// offset border_points position based on position and line size
	if ((left && outer) || (!left && !outer)) {
		// shift points left
		x -= line_size.x;
	}
	else {
		// shift points right
		x += line_size.x;
	}

	if ((top && outer) || (!top && !outer)) {
		// shift points up
		y -= line_size.y;
	}
	else {
		// shift poitns down
		y += line_size.y;
	}


	gl_Position = vec4(x, y, 0.0, 1.0);
}
