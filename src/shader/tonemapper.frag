#version 450

layout (input_attachment_index = 0, binding = 0) uniform subpassInput gbufferPosition;
layout (input_attachment_index = 1, binding = 1) uniform subpassInput gbufferNormal;
layout (input_attachment_index = 2, binding = 2) uniform subpassInput gbufferAlbedo;
layout (input_attachment_index = 3, binding = 3) uniform subpassInput gbufferRoughness;
layout (input_attachment_index = 4, binding = 4) uniform subpassInput gbufferMetallic;
layout (input_attachment_index = 0, binding = 5) uniform subpassInput inputDiffuse;
layout (input_attachment_index = 0, binding = 6) uniform subpassInput inputSpecular;

layout(set = 0, binding = 7) uniform usampler2D occlusion_buffer;

layout (location = 0) out vec4 swapchain_out;
layout (location = 1) out vec4 scene_color;
layout (location = 2) out uint luma_out;

layout(push_constant) uniform Constants {
    uint debug_vis_mode;
    vec2 screen_dimensions;
    float exposure_adjustment;
    float vignette_opacity;
} constants;

#include "constants.inc"
#include "debug_vis.inc"

void main() {
    // pipeline luminance to absolute luminance
    vec3 diffuse = subpassLoad(inputDiffuse).rgb * INTERNAL_HDR_DIV;
    vec3 specular = subpassLoad(inputSpecular).rgb * INTERNAL_HDR_DIV;
    vec3 hdrColor = diffuse + specular;
    scene_color = vec4(hdrColor, 1.0);

    float fragment_luma = dot(diffuse, LUMA_COMPONENTS);
    // uint conversion offset
    fragment_luma *= 1000.0;
    luma_out = uint(fragment_luma);

    vec2 center = vec2(constants.screen_dimensions[0] / 2, constants.screen_dimensions[1] / 2);
    vec2 distance = abs(gl_FragCoord.xy - center) / center;
    float vignette_amount = smoothstep(0.0, 1.0, length(distance * 0.707));
    float vignette = 1.0 - (vignette_amount * constants.vignette_opacity);

    vec3 tonemapped = hdrColor * 1.0 *constants.exposure_adjustment * vignette;
    swapchain_out = vec4(tonemapped, 1.0);

    if (constants.debug_vis_mode == DEBUG_VISUALIZE_POSITION_BUFFER) {
        vec3 frag_pos = subpassLoad(gbufferPosition).rgb;
        swapchain_out = vec4(frag_pos / 100.0, 1.0);
    }
    else if (constants.debug_vis_mode == DEBUG_VISUALIZE_NORMAL_BUFFER) {
        vec3 N = normalize(subpassLoad(gbufferNormal).rgb);
        swapchain_out = vec4(N, 1.0);
    }
    else if (constants.debug_vis_mode == DEBUG_VISUALIZE_ALBEDO_BUFFER) {
        vec3 albedo = subpassLoad(gbufferAlbedo).rgb;
        swapchain_out = vec4(albedo, 1.0);
    }
    else if (constants.debug_vis_mode == DEBUG_VISUALIZE_ROUGHNESS_BUFFER) {
        float roughness = subpassLoad(gbufferRoughness).r;
        swapchain_out = vec4(vec3(roughness), 1.0);
    }
    else if (constants.debug_vis_mode == DEBUG_VISUALIZE_METALLIC_BUFFER) {
        float metallic = subpassLoad(gbufferMetallic).r;
        swapchain_out = vec4(vec3(metallic), 1.0);
    }
    else if (constants.debug_vis_mode == DEBUG_VISUALIZE_DIFFUSE_LIGHTING_ONLY) {
        swapchain_out = vec4(diffuse, 1.0);
    }
    else if (constants.debug_vis_mode == DEBUG_VISUALIZE_SPECULAR_LIGHTING_ONLY) {
        swapchain_out = vec4(specular, 1.0);
    }
    else if (constants.debug_vis_mode == DEBUG_VISUALIZE_OCCLUSION_BUFFER) {
        vec2 uv = vec2(gl_FragCoord.x / constants.screen_dimensions[0],
        gl_FragCoord.y / constants.screen_dimensions[1]);
        uvec4 occlusion_id = texture(occlusion_buffer, uv);
        uint full_id = occlusion_id[3] + occlusion_id[2] + occlusion_id[1] + occlusion_id[0];
        float occlusion_normalized = mod(full_id, 256) / 256.0;
        vec3 color = (tonemapped * 0.333) + (vec3(occlusion_normalized) * 0.666);
        swapchain_out = vec4(vec3(occlusion_normalized), 1.0);
    }
    else if (constants.debug_vis_mode == DEBUG_VISUALIZE_NO_POST_PROCESSING) {
        // passthrough
        swapchain_out = vec4(hdrColor / INTERNAL_HDR_DIV, 1.0);
    }
}
