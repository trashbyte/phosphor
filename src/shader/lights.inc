#include "util.inc"
#include "bsdf.inc"

vec3 hemisphere_light(const in vec3 normal, const in vec3 lightDirection, const in vec3 sky, const in vec3 ground) {
    float weight = 0.5 * dot(normalize(normal), lightDirection) + 0.5;
    return mix(ground, sky, weight);
}

void point_light(vec3 light_pos, vec3 light_color, vec3 N, vec3 V,
                 vec3 albedo, float roughness, float metallic, vec3 frag_pos,
                 inout vec3 diffuse_out, inout vec3 specular_out) {
    vec3 L = normalize(light_pos - frag_pos);
    vec3 H = normalize(V + L);

    float distance = length(light_pos - frag_pos);
    float attenuation = 1.0 / (distance * distance);
    vec3 radiance = light_color * attenuation;

    // F: Cook-Torrance specular term
    vec3 F0 = vec3(0.04);
    F0 = mix(F0, albedo, metallic);
    vec3 F = FresnelSchlick(max(dot(H, V), 0.0), F0);

    // NDF: Normal distribution function (Trowbridge-Reitz GGX)
    float NDF = DistributionGGX(N, H, roughness);

    // G: Geometry function (Schlick-GGX)
    float G = GeometrySmith(N, V, L, roughness);

    // ratio of refraction
    vec3 kS = F;
    vec3 kD = vec3(1.0) - kS;
    // metallic surfaces don't refract
    kD *= 1.0 - metallic;

    // calculating final Cook-Torrance BRDF value
    vec3 numerator = NDF * G * F;
    float denominator = 4.0 * max(dot(N, V), 0.0) * max(dot(N, L), 0.0);
    vec3 specular = numerator / max(denominator, 0.001);

    float NdotL = max(dot(N, L), 0.0);
    diffuse_out += (kD * albedo / PI) * radiance * NdotL;
    specular_out += specular * radiance * NdotL;
}

vec3 directional_light(vec3 L, vec3 light_color, vec3 N, vec3 V, vec3 albedo, float roughness, float metallic, vec3 frag_pos) {
    L = -L; // direction of light -> direction to light
    vec3 H = normalize(V + L);

    // F: Cook-Torrance specular term
    vec3 F0 = vec3(0.04);
    F0 = mix(F0, albedo, metallic);
    vec3 F = FresnelSchlick(max(dot(H, V), 0.0), F0);

    // NDF: Normal distribution function (Trowbridge-Reitz GGX)
    float NDF = DistributionGGX(N, H, roughness);

    // G: Geometry function (Schlick-GGX)
    float G = GeometrySmith(N, V, L, roughness);

    // ratio of refraction
    vec3 kS = F;
    vec3 kD = vec3(1.0) - kS;
    // metallic surfaces don't refract
    kD *= 1.0 - metallic;

    // calculating final Cook-Torrance BRDF value
    vec3 numerator = NDF * G * F;
    float denominator = 4.0 * max(dot(N, V), 0.0) * max(dot(N, L), 0.0);
    vec3 specular = numerator / max(denominator, 0.001);

    float NdotL = max(dot(N, L), 0.0);
    // final radiance value
    return (kD * albedo / PI + specular) * light_color * NdotL;
}
