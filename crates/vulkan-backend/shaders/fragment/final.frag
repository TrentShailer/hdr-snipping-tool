#version 450

layout(location = 0) in vec2 in_uv;
layout(location = 0) out vec4 out_color;

layout(input_attachment_index = 0, set = 0, binding = 0) uniform subpassInput capture_out;
layout(input_attachment_index = 1, set = 0, binding = 1) uniform subpassInput selection_out;
layout(input_attachment_index = 2, set = 0, binding = 2) uniform subpassInput border_out;
layout(input_attachment_index = 3, set = 0, binding = 3) uniform subpassInput mouse_out;

void main() {
    vec4 capture = subpassLoad(capture_out);
    vec4 selection = subpassLoad(selection_out);
    vec4 border = subpassLoad(border_out);
    vec4 mouse = subpassLoad(mouse_out);

    vec3 output_rgb = mix(capture.rgb, selection.rgb, selection.a);

    if (mouse == vec4(1.0, 1.0, 1.0, 1.0)) {
        output_rgb = mix(output_rgb, vec3(1.0) - output_rgb, 0.5);
    }

    output_rgb = mix(output_rgb.rgb, border.rgb, border.a);
    
    out_color = vec4(output_rgb, 1.0);
    
}