
fn getKey(pos: vec3u) -> vec2u {
  return vec2u(
    extractBits(pos.x, 0u, 21u) << 11 + extractBits(pos.y, 0u, 11u),
    extractBits(pos.z, 0u, 21u) << 11 + extractBits(pos.y, 11u, 11u),
  );
}

fn locationHint(key: vec2u) -> u32 {
  return key.x + key.y;
}
