import package::commons::{ commons, get_random, tau, pi };
import package::hashmap::write as hashmap;
import package::model;

override block_size: u32 = 16;

struct Pc {
	mvp: mat4x4<f32>,
}
var<immediate> pc: Pc;

@compute @workgroup_size(block_size, block_size, 1)
fn main(@builtin(global_invocation_id) grid: vec3u) {
	var orig = (pc.mvp * vec4(0.0, 0.0, 0.0, 1.0)).xyz;
	var targ = (pc.mvp * vec4(f32(grid.x), f32(grid.y), -1.0, 1.0)).xyz;
	var dir = normalize(targ - orig);
	var color = vec3f(0.0, 0.0, 0.0);
	var mask = vec3f(1.0);
	var dist = 0.0;
	var first_hit = vec3u();

	for (var i = 0u; i < 3; i++) {
		let hit = model::ray_cast(orig, dir);

		if !hit.hit {
			break;
		}

		if i == 0 {
			first_hit = hit.pos;
		}

		dist += hit.t;
		let hit_mat = model::model.palette[hit.voxel];
		let light = 1.0 / (1.0 + dist / 4.0);

		if hit_mat.lum > 0.0 {
			color += mask * hit_mat.col.rgb * hit_mat.lum * light;
			break;
		}

		mask *= hit_mat.col.rgb;

		if all(mask <= vec3f(1.0 / 128.0)) {
			break;
		}

		let random = get_random(vec3u(grid.x, grid.y, commons.frame * 3 + i));

		orig += dir * (hit.t - 0.01);

		let yaw = random.x * tau;
		let pitch = pi / 2 - acos(random.y);
		let up_guess = select(vec3f(0.0, 1.0, 0.0), vec3f(1.0, 0.0, 0.0), abs(hit.norm.y) >= 0.9);
		let right = normalize(cross(hit.norm, up_guess));
		let front = normalize(cross(hit.norm, right));

		dir = hit.norm * sin(pitch) + right * cos(pitch) * cos(yaw) + front * cos(pitch) * sin(yaw);
	}
	
	if any(first_hit != vec3u()) {
		let key = hashmap::getKey(first_hit);
		let write_pos = hashmap::insert(key);
		if(write_pos != hashmap::NOT_FOUND) {
			let write_value = vec4u(vec3u(saturate(color) * 256.0), 1);
			atomicAdd(&hashmap::hashmap[write_pos + 0], write_value.r);
			atomicAdd(&hashmap::hashmap[write_pos + 1], write_value.g);
			atomicAdd(&hashmap::hashmap[write_pos + 2], write_value.b);
			atomicAdd(&hashmap::hashmap[write_pos + 3], write_value.a);
		}
	}
}