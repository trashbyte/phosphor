#version 450

layout (input_attachment_index = 0, binding = 0) uniform subpassInput inputDiffuse;
layout (input_attachment_index = 0, binding = 1) uniform subpassInput inputSpecular;

layout (location = 0) out vec4 scene_color;
layout (location = 1) out uint luma_out;

#include "constants.inc"

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
}
