out uint frag;

uniform usampler2D current_gen_texture;

// This code is ass stupid and I’m sure there’s a way to optimize that to yield less texture taps.
// Probably a box blur should work, which is supported by default in GLSL, but I haven’t tried
// (simply divide by 9 afterwards).
uint count_alive_neighbors() {
  return texelFetch(current_gen_texture, ivec2(gl_FragCoord.x - 1, gl_FragCoord.y + 1), 0).r
  + texelFetch(current_gen_texture, ivec2(gl_FragCoord.x , gl_FragCoord.y + 1), 0).r
  + texelFetch(current_gen_texture, ivec2(gl_FragCoord.x + 1, gl_FragCoord.y + 1), 0).r
  + texelFetch(current_gen_texture, ivec2(gl_FragCoord.x - 1, gl_FragCoord.y), 0).r
  + texelFetch(current_gen_texture, ivec2(gl_FragCoord.x + 1, gl_FragCoord.y), 0).r
  + texelFetch(current_gen_texture, ivec2(gl_FragCoord.x - 1, gl_FragCoord.y - 1), 0).r
  + texelFetch(current_gen_texture, ivec2(gl_FragCoord.x , gl_FragCoord.y - 1), 0).r
  + texelFetch(current_gen_texture, ivec2(gl_FragCoord.x + 1, gl_FragCoord.y - 1), 0).r;
}

void main() {
  uint state = texelFetch(current_gen_texture, ivec2(gl_FragCoord.xy), 0).r;
  uint alive_neighbors = count_alive_neighbors();

  frag = uint(alive_neighbors == 3u || state == 1u && alive_neighbors == 2u);
}
