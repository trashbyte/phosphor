#version 450

layout (input_attachment_index = 0, binding = 0) uniform subpassInput gbufferPosition;
layout (input_attachment_index = 1, binding = 1) uniform subpassInput gbufferNormal;
layout (input_attachment_index = 2, binding = 2) uniform subpassInput gbufferAlbedo;
layout (input_attachment_index = 3, binding = 3) uniform subpassInput gbufferRoughness;
layout (input_attachment_index = 4, binding = 4) uniform subpassInput gbufferMetallic;

layout(location = 0) out vec4 f_color;

layout(push_constant) uniform Constants {
    mat4 view;
    vec3 view_pos;
    uint debug_vis_mode;
} constants;

#include "lights.inc"
#include "debug_vis.inc"

void main() {
    vec3 light_positions[3];
    vec3 light_colors[3];

    light_positions[0] = vec3(16.0, 26.0, 16.0);
    light_colors[0] = vec3(0.2, 0.4, 1.0) * 50.0;

    light_positions[1] = vec3(96.0, 14.0, 14.0);
    light_colors[1] = vec3(1.0, 0.7, 0.3) * 1000.0;

    light_positions[2] = vec3(64.0, 40.0, -64.0);
    light_colors[2] = vec3(1.0, 0.2, 0.4) * 500.0;

    vec3 frag_pos = subpassLoad(gbufferPosition).rgb;
    vec3 N = subpassLoad(gbufferNormal).rgb;
    vec3 V = normalize(constants.view_pos - frag_pos);
    vec3 albedo = subpassLoad(gbufferAlbedo).rgb;
    float roughness = 0.9;//subpassLoad(gbufferRoughness).r;
    float metallic = subpassLoad(gbufferMetallic).r;

    // summing irradiance for all lights
    vec3 Lo = vec3(0.0);
    for(int i = 0; i < 3; ++i) {
        Lo += point_light(light_positions[i], light_colors[i], N, V, albedo, roughness, metallic, frag_pos);
    }
    Lo += directional_light(normalize(vec3(0.5, -1.0, 0.5)), vec3(1.0, 1.0, 0.9) * 5.0, N, V, albedo, roughness, metallic, frag_pos);

    //vec3 hemi = hemisphere_light(N, vec3(0.0,1.0,0.0), vec3(0.9,0.9,1.0), vec3(0.0,0.0,0.0));
    vec3 color = Lo + (albedo * 0.1);
    // absolute luminance to pipeline luminance
    color = color / INTERNAL_HDR_DIV;

    if (constants.debug_vis_mode == DEBUG_VISUALIZE_POSITION_BUFFER) {
        f_color = vec4(frag_pos / 100.0, 1.0);
    }
    else if (constants.debug_vis_mode == DEBUG_VISUALIZE_NORMAL_BUFFER) {
        f_color = vec4(N, 1.0);
    }
    else if (constants.debug_vis_mode == DEBUG_VISUALIZE_ALBEDO_BUFFER) {
        f_color = vec4(albedo, 1.0);
    }
    else if (constants.debug_vis_mode == DEBUG_VISUALIZE_ROUGHNESS_BUFFER) {
        f_color = vec4(vec3(roughness), 1.0);
    }
    else if (constants.debug_vis_mode == DEBUG_VISUALIZE_METALLIC_BUFFER) {
        f_color = vec4(vec3(metallic), 1.0);
    }
    else if (constants.debug_vis_mode == DEBUG_VISUALIZE_DEFERRED_LIGHTING_ONLY) {
        f_color = vec4(Lo, 1.0);
    }
    else {
        f_color = vec4(color, 1.0);
    }
}
