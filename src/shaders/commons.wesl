import package::hashmap;

@group(0) @binding(0) var<uniform> commons: Commons;
@group(0) @binding(1) var entropy_tex: texture_2d<f32>;

const pi = radians(180.0);
const tau = radians(360.0);

struct Commons {
	frame: u32,
}

fn hash(x: u32) -> u32 {
	var z = x + 0x9e3779b9u;
	z = (z ^ (z >> 16u)) * 0x85ebca6bu;
	z = (z ^ (z >> 13u)) * 0xc2b2ae35u;
	z = z ^ (z >> 16u);
	return z;
}

fn get_random(coords: vec3u) -> vec2f {
	let dims = textureDimensions(entropy_tex);
	let x = hash(hash(coords.x) ^ coords.z) % dims.x;
	let y = hash(hash(coords.y) ^ coords.z) % dims.y;
	return textureLoad(entropy_tex, vec2u(x, y), 0).xy;
}
