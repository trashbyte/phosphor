#version 450

layout(set = 0, binding = 0) uniform sampler2D tex;

layout(location = 0) in vec2 f_uv;
layout(location = 1) in vec4 f_color;

layout(location = 0) out vec4 out_color;

void main() {
  out_color = f_color * texture(tex, f_uv.st);
}
