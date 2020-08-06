in vec2 v_ss_pos;

out vec4 frag;

uniform usampler2D source;
uniform vec2 scale_ratio;

const float PI = 3.14159265;

void main() {
  float v = texelFetch(source, ivec2(gl_FragCoord.xy * scale_ratio), 0).r;
  float c = cos(PI * (1. + v_ss_pos.x));
  float s = sin(PI * (1. + v_ss_pos.x));

  vec3 color_a = vec3(1., .3, .5) * v;
  vec3 color_b = vec3(.5, .3, 1.) * v;

  frag = vec4(mix(color_a, color_b, 1. - abs(v_ss_pos.x)), 1.);
}
