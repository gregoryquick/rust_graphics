#version 450

layout(location=0) in vec3 v_color;
layout(location=1) in vec2 v_position;

layout(location=0) out vec4 f_color;

void main() {
    float alpha = 1.0 - 4.0*dot(v_position, v_position);
    f_color = vec4(alpha*v_color, 1.0);
}
