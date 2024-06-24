#version 450

layout(location = 0) in vec2 position;
layout(location = 1) in vec4 color;
layout(location = 2) in uint flags;
// locked_bit, top bit, left bit

layout(location = 0) out vec4 out_color;

layout(push_constant) uniform PushConstants {
    vec4 selection; // ltrb, scaled to be in vulkan coordinates
};

void main() {
	bool locked = (flags & 1 << 2) >> 2 == 1;
	bool top 	= (flags & 1 << 1) >> 1 == 1;
	bool left 	= (flags & 1 << 0) >> 0 == 1;

    out_color = color;

	if (locked) {
    	gl_Position = vec4(position, 0.0, 1.0);
	}
	else {
		// set position = selection position
		float x = 0.0f;
		float y = 0.0f;

		if (top) {
			y = selection[1];
		}
		else {
			y = selection[3];
		}

		if (left) {
			x = selection[0];
		}
		else {
			x = selection[2];
		}


		gl_Position = vec4(x, y, 0.0, 1.0);
	}

}
