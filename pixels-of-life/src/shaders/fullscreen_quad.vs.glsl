out vec2 v_ss_pos;

// Fullscreen quad.
const vec2[4] FULLSCREEN_QUAD = vec2[](
  vec2(-1., -1.),
  vec2( 1., -1.),
  vec2( 1.,  1.),
  vec2(-1.,  1.)
);

void main() {
  v_ss_pos = FULLSCREEN_QUAD[gl_VertexID];
  gl_Position = vec4(v_ss_pos, 0., 1.);
}
