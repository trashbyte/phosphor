#version 450

#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable

layout (location = 0) in vec3 position;

layout(location = 0) out vec4 out_color;

layout(push_constant) uniform Constants {
	mat4 matrix;
	float sun_rotation;
	float sun_transit;
} constants;

void main() {
	gl_Position = constants.matrix * vec4(position.xyz, 0.0);
	out_color = vec4(0.8, 0.9, 1.0, 1.0);
}
