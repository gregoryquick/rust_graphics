#version 450

layout(location=0) in vec4 aPosition;

void main() {
    gl_Position = aPosition;
}
