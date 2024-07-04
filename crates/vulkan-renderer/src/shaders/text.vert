#version 450

layout(location = 0) in vec2 position;
layout(location = 1) in vec2 uv;
layout(location = 2) in vec4 color;

// per instance data
layout(location = 3) in vec2 glyph_position; // in px units
layout(location = 4) in vec2 glyph_size; // in px units
layout(location = 5) in vec2 bitmap_size; // in px units
layout(location = 6) in vec2 uv_offset; // scaled

layout(push_constant) uniform PushConstants {
	vec2 base_position;
	vec2 base_size;

	vec2 text_size;
    vec2 text_position;
	vec2 window_size; // in px
	float atlas_dim;
};

layout(location = 0) out vec2 out_uv;
layout(location = 1) out vec4 out_color;

void main() {
	out_color = color;

	vec2 bitmap_scale = bitmap_size / atlas_dim;
	out_uv = uv * bitmap_scale + uv_offset;

	vec2 target_size = (glyph_size / window_size) * 2.0;
	vec2 glyph_top_left = (glyph_position / window_size) * 2.0; // Target position for the glyph in the 'text' space (relative to first glyph top left)
	vec2 glyph_centre = glyph_top_left + target_size / 2.0; // Find the centre of the glyph
	vec2 target_position = glyph_centre + text_position - (text_size / 2.0);

	// calcuate the scale that should be applied to the rect
	vec2 scale = target_size / base_size;
	vec2 scaled_position = position * scale;

	// Work out the position offset to move the rect to the target position
	vec2 position_offset = target_position - base_position;

	vec2 out_position = scaled_position + position_offset;

	gl_Position = vec4(out_position, 0.0, 1.0);
}
