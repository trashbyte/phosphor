#version 450

layout(push_constant) uniform Constants {
  mat4 matrix;
} constants;

layout(location = 0) in vec2 pos;
layout(location = 1) in vec2 uv;
layout(location = 2) in vec4 col;

layout(location = 0) out vec2 f_uv;
layout(location = 1) out vec4 f_color;

void main() {
  f_uv = uv;
  // need to reverse-gamma correct since the swapchain is already accounting for sRGB
  f_color = pow(col / 255.0, vec4(2.2, 2.2, 2.2, 1.0));
  gl_Position = constants.matrix * vec4(pos.xy, 0, 1);
}
