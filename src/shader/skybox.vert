#version 450

layout (location = 0) in vec3 position;
layout (location = 1) in vec2 uv;

layout(location = 0) out vec2 out_uv;

layout(push_constant) uniform Constants {
	mat4 matrix;
	float sun_rotation;
	float sun_transit;
} constants;

void main() {
	gl_Position = constants.matrix * vec4(position.xyz, 0.0);
	out_uv = vec2( uv.x, -abs(uv.y - 0.5) + 0.5 );
}
