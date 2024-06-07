#version 450

layout(location = 0) in vec2 tex_coords;
layout(location = 0) out vec4 f_color;

layout(set = 0, binding = 0) uniform sampler s;
layout(set = 0, binding = 1) uniform texture2D tex;

layout(push_constant) uniform PushConstants {
	ivec2 mouse_position;
	uvec4 selection;
};

bool is_outside_selection(ivec2 position) {
	// Because left and top px 0 is often not shown on the display
	uint left = selection[0];
	if (left == 0) {
		left = 1;
	}

	uint top = selection[1];
	if (top == 0) {
		top = 1;
	}

	uint right = selection[2];
	uint bottom = selection[3];

	return position.x < left || position.x > right || position.y < top || position.y > bottom;
}

bool is_selection_border(ivec2 position) {
	if (is_outside_selection(position)) {
		return false;
	}

	// Because left and top px 0 is often not shown on the display
	uint left = selection[0];
	if (left == 0) {
		left = 1;
	}

	uint top = selection[1];
	if (top == 0) {
		top = 1;
	}

	uint right = selection[2];
	uint bottom = selection[3];

	return position.x == left || position.x == right || position.y == top || position.y == bottom;
}

bool is_mouse_guide(ivec2 position) {
	int x = mouse_position.x;
	if (x == 0) {
		x = 1;
	}

	int y = mouse_position.y;
	if (y == 0) {
		y = 1;
	}

	return position.x == x || position.y == y;
}

void main() {
	vec2 texture_size = vec2(textureSize(sampler2D(tex, s), 0));
	ivec2 position = ivec2(ceil(tex_coords * texture_size));
	vec4 texture_color = texture(sampler2D(tex, s), tex_coords);
	vec4 output_color = texture_color;

	if (is_selection_border(position)) {
		output_color = vec4(vec3(0.5), 1.0);
	}
	else if (is_mouse_guide(position)){
		output_color = vec4(0.75 * (1.0 - texture_color.rgb), texture_color.a);
	}
	else if (is_outside_selection(position)) {
		output_color = vec4(0.5 * texture_color.rgb, texture_color.a);
	}

    f_color = output_color;
}
