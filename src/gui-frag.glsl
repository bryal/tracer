uniform sampler2D u_sampler;

in vec4 v_color;
in vec2 v_tc;

out vec4 f_color;

void main() {
    f_color = v_color;
    f_color.a *= texture(u_sampler, v_tc).r;
}
