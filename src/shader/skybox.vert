#version 450

layout (location = 0) in vec3 position;
layout (location = 1) in vec3 normal;
layout (location = 2) in vec3 tangent;
layout (location = 3) in vec2 uv;

//layout(location = 0) out vec3 out_position;
//layout(location = 1) out vec3 out_normal;
//layout(location = 2) out vec2 out_uv;

layout(push_constant) uniform Constants {
	mat4 matrix;
	float sun_rotation;
	float sun_transit;
} constants;

void main() {
	gl_Position = /*constants.matrix * */ vec4(position, 0.0);
//	out_position = position;
//	out_normal = normal;
//	out_uv = vec2( uv.x, -abs(uv.y - 0.5) + 0.5 );
}
