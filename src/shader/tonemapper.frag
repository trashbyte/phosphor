#version 450

layout (input_attachment_index = 0, set = 0, binding = 0) uniform subpassInput inputColor;
layout(set = 0, binding = 1) uniform usampler2D occlusion_buffer;

layout (location = 0) out vec4 f_color;
layout (location = 1) out float luma_out;

layout(push_constant) uniform Constants {
    uint debug_vis_mode;
    vec2 screen_dimensions;
    float exposure_adjustment;
    float vignette_opacity;
} constants;

const float E = 2.71828;

#include "constants.inc"
#include "debug_vis.inc"

void main() {
    // pipeline luminance to absolute luminance
    vec3 hdrColor = subpassLoad(inputColor).rgb * INTERNAL_HDR_DIV;
    float fragment_luma = dot(hdrColor, vec3(0.2126, 0.7152, 0.0722));

    vec2 center = vec2(constants.screen_dimensions[0] / 2, constants.screen_dimensions[1] / 2);
    vec2 distance = abs(gl_FragCoord.xy - center) / center;
    float vignette_amount = smoothstep(0.0, 1.0, length(distance * 0.707));
    float vignette = 1.0 - (vignette_amount * constants.vignette_opacity);

    vec3 tonemapped = hdrColor * constants.exposure_adjustment * vignette;
    f_color = vec4(tonemapped, 1.0);

    if (constants.debug_vis_mode != 0) {
        if (constants.debug_vis_mode == DEBUG_VISUALIZE_OCCLUSION_BUFFER) {
            vec2 uv = vec2(gl_FragCoord.x / constants.screen_dimensions[0],
                           gl_FragCoord.y / constants.screen_dimensions[1]);
            uvec4 occlusion_id = texture(occlusion_buffer, uv);
            uint full_id = occlusion_id[3] + occlusion_id[2] + occlusion_id[1] + occlusion_id[0];
            float occlusion_normalized = mod(full_id, 256) / 256.0;
            vec3 color = (tonemapped * 0.333) + (vec3(occlusion_normalized) * 0.666);
            f_color = vec4(vec3(occlusion_normalized), 1.0);
        }
        else {
            // passthrough
            f_color = vec4(subpassLoad(inputColor).rgb, 1.0);
        }
    }

    if (gl_FragCoord.z < 1000.0) {
        // pipeline luminance to absolute luminance
        luma_out = fragment_luma * INTERNAL_HDR_DIV;
    }
    else {
        luma_out = -1.0;
    }
}
