uniform vec2 u_screen_size;
uniform vec2 u_tex_size;

in vec2 a_pos;
in vec4 a_color;
in ivec2 a_tc;

out vec4 v_color;
out vec2 v_tc;

void main() {
    // Draw GUI over traced image
    float z = -1.0;
    gl_Position = vec4(
	2.0 * a_pos.x / u_screen_size.x - 1.0,
	1.0 - 2.0 * a_pos.y / u_screen_size.y,
	z,
	1.0);
    v_color = a_color;
    v_tc = a_tc / u_tex_size;
}
