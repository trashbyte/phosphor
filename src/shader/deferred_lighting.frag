#version 450

layout (input_attachment_index = 0, binding = 0) uniform subpassInput gbufferPosition;
layout (input_attachment_index = 1, binding = 1) uniform subpassInput gbufferNormal;
layout (input_attachment_index = 2, binding = 2) uniform subpassInput gbufferAlbedo;
layout (input_attachment_index = 3, binding = 3) uniform subpassInput gbufferRoughness;
layout (input_attachment_index = 4, binding = 4) uniform subpassInput gbufferMetallic;

layout (set = 0, binding = 5) uniform sampler2D irrCubemap;
layout (set = 0, binding = 6) uniform sampler2D radCubemap;
layout (set = 0, binding = 7) uniform sampler2D brdfLookup;

layout(location = 0) out vec4 diffuse_out;
layout(location = 1) out vec4 specular_out;

layout(push_constant) uniform Constants {
    mat4 view;
    vec3 view_pos;
    uint debug_vis_mode;
} constants;

#include "lights.inc"

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
    vec3 N = normalize(subpassLoad(gbufferNormal).rgb);
    vec3 V = normalize(constants.view_pos - frag_pos);
    vec3 R = reflect(-V, N);
    vec3 albedo = subpassLoad(gbufferAlbedo).rgb;
    float roughness = 0.9;//subpassLoad(gbufferRoughness).r;
    float metallic = subpassLoad(gbufferMetallic).r;

    // irradiance for point lights
    vec3 point_lights_diff = vec3(0.0);
    vec3 point_lights_spec = vec3(0.0);
    for(int i = 0; i < 3; ++i) {
        point_light(light_positions[i], light_colors[i], N, V, albedo, roughness, metallic, frag_pos, point_lights_diff, point_lights_spec);
    }
    //Lo += directional_light(normalize(vec3(0.5, -1.0, 0.5)), vec3(1.0, 1.0, 0.9) * 5.0, N, V, albedo, roughness, metallic, frag_pos);

    // specular coefficient
    vec3 F0 = vec3(0.04);
    F0 = mix(F0, albedo, metallic);
    vec3 F = FresnelSchlickRoughness(max(dot(N, V), 0.0), F0, roughness);
    vec3 kS = F;
    vec3 kD = 1.0 - kS;
    kD *= 1.0 - metallic;

    // equirectangular UVs from normal
    vec2 uv = vec2(atan(N.z, N.x), acos(N.y));
    uv /= vec2(2 * PI, PI);

    // diffuse irradiance
    vec3 irradiance = texture(irrCubemap, uv).rgb;
    vec3 diffuse    = irradiance * albedo;
    vec3 ibl_diffuse    = (kD * diffuse); // * ao;

    // equirectangular UVs from reflected normal
    uv = vec2(atan(R.z, R.x), acos(R.y));
    uv /= vec2(2 * PI, PI);

    // specular radiance
    const float MAX_REFLECTION_LOD = 4.0;
    vec3 prefilteredColor = textureLod(radCubemap, uv,  roughness * MAX_REFLECTION_LOD).rgb;
    vec2 envBRDF  = texture(brdfLookup, vec2(max(dot(N, V), 0.0), roughness)).rg;
    vec3 ibl_specular = prefilteredColor * (F * envBRDF.x + envBRDF.y);

    // absolute luminance to pipeline luminance
    diffuse_out = vec4(vec3((point_lights_diff + ibl_diffuse) / INTERNAL_HDR_DIV), 1.0);
    specular_out = vec4(vec3((point_lights_spec + ibl_specular) / INTERNAL_HDR_DIV), 1.0);
}
