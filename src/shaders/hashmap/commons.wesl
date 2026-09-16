import package::commons::hash;

const ENTRY_SIZE: u32 = 6;
const NOT_FOUND: u32 = u32(i32(-1));

fn getKey(pos: vec3u) -> vec2u {
	return vec2u(
		(extractBits(pos.x, 0u, 21u) << 11) + extractBits(pos.y, 0u, 11u) + 1,
		(extractBits(pos.z, 0u, 21u) << 11) + extractBits(pos.y, 11u, 11u) + 1,
	);
}

fn locationHint(key: vec2u) -> u32 {
	return hash(key.x ^ hash(key.y));
}
