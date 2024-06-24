#version 450

layout(location = 0) in vec2 in_uv;

layout(location = 0) out vec4 out_color;

layout(set = 0, binding = 0) uniform sampler capture_sampler;
layout(set = 0, binding = 1) uniform texture2D capture;

layout(push_constant) uniform PushConstants {
    ivec4 selection; // ltrb
    ivec2 mouse_position;
    int line_width;
};

bool near(int position, int target) {
    // < 0 for left/above of line
    int diff = abs(position - target);

    return diff <= line_width;
}

bool outside_selection(ivec2 position) {
    return  position.x < selection[0] ||
            position.x > selection[2] || 
            position.y < selection[1] || 
            position.y > selection[3];
}

bool mouse_guide(ivec2 position) {
    return near(position.x, mouse_position.x) || near(position.y, mouse_position.y);
}

bool selection_border(ivec2 position) {
    // left
    if (
        near(position.x, selection[0]) &&
        position.y >= selection[1] &&
        position.y <= selection[3]
    ) {
        return true;
    }
    
    // top
    if (
        near(position.y, selection[1]) &&
        position.x >= selection[0] &&
        position.x <= selection[2]
    ) {
        return true;
    }

    // right
    if (
        near(position.x, selection[2]) &&
        position.y >= selection[1] &&
        position.y <= selection[3]
    ) {
        return true;
    }
    
    // botton
    if (
        near(position.y, selection[3]) &&
        position.x >= selection[0] &&
        position.x <= selection[2]
    ) {
        return true;
    }


    return false;
}

void main() {
    ivec2 capture_size = textureSize(sampler2D(capture, capture_sampler), 0);

    // convert uv coordinates to capture space
    ivec2 position = ivec2(float(capture_size.x) * in_uv.x, float(capture_size.y) * in_uv.y);
    // ivec2 position = ivec2(gl_FragCoord.xy); // gl_FragCoord assumes bottom right indexing

    vec3 output_rgb = texture(sampler2D(capture, capture_sampler), in_uv).rgb;

    // Darken pixels outside the selection
    if (outside_selection(position)) {
        output_rgb = output_rgb * 0.5;
    }

    // Invert pixels for mouse guide
    if (mouse_guide(position)) {
        output_rgb = vec3(1.0) - output_rgb;
    }

    // Draw selection border
    if (selection_border(position)) {
        output_rgb = vec3(0.8);
    }

    out_color = vec4(output_rgb, 1.0);
}
