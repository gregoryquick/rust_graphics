#version 450
#extension GL_ARB_separate_shader_objects : enable

layout(push_constant) uniform PushConstants {
    vec4 color;
    vec2 p1;
    vec2 p2;
    vec2 p3;
} push_constants;

layout(location = 0) out vec4 vertex_color;

vec2 positions[3] = vec2[](
    push_constants.p1,
    push_constants.p2,
    push_constants.p3
);

void main() {
    vec2 pos = positions[gl_VertexIndex];
    vertex_color = push_constants.color;
    gl_Position = vec4(pos, 0.0, 1.0);
}
