#version 450

layout(location = 0) in vec2 position;
layout(location = 1) in vec4 color;
layout(location = 2) in uint flags;
// left/top bit, positive shift bit, vertical bit

layout(location = 0) out vec4 out_color;

layout(push_constant) uniform PushConstants {
    vec2 mouse_position; // scaled to be in vulkan coordinates
	vec2 line_size; // scaled to be in vulkan coordinates
};

void main() {
	bool left_or_top 	= (flags & 1 << 2) >> 2 == 1;
	bool positive_shift = (flags & 1 << 1) >> 1 == 1;
	bool vertical 		= (flags & 1 << 0) >> 0 == 1;

    out_color = color;

	// Get position for this vertex
	float x = 0.0f;
	float y = 0.0f;

	if (left_or_top && vertical) {
		y = -1;
		x = mouse_position.x;
	}
	else if (!left_or_top && vertical) {
		y = 1;
		x = mouse_position.x;
	}
	else if (left_or_top && !vertical) {
		y = mouse_position.y;
		x = -1;
	}
	else if (!left_or_top && !vertical) {
		y = mouse_position.y;
		x = 1;
	}

	// offset border_points position based on position and line size
	if (positive_shift && vertical) {
		// offset x by line +x
		x += line_size.x / 2.0;
	}
	else if (!positive_shift && vertical) {
		// offset x by line -x
		x -= line_size.x / 2.0;
	}
	else if (positive_shift && !vertical) {
		// offset y by line +y
		y += line_size.y / 2.0;
	}
	else if (!positive_shift && !vertical) {
		// offset y by line -y
		y -= line_size.y / 2.0;
	}

	gl_Position = vec4(x, y, 0.0, 1.0);
}
