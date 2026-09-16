
@group(2) @binding(0) var<storage> model: Model;

struct Model {
	size: vec4i,
	palette: array<Material, 256>,
	voxels: array<u32>,
}

struct Material {
	col: vec4f,
	lum: f32,
	rough: f32,
	ior: f32,
	metal: f32,
}

fn find_voxel(pos: vec3i) -> u32 {
	let index = u32(pos.x + pos.y * model.size.x + pos.z * model.size.x * model.size.y);
	return extractBits(model.voxels[index / 4], index % 4 * 8, 8u);
}

struct BoxIntersection {
	hit: bool,
	t_min: f32,
	t_max: f32,
}

fn box_intersection(orig: vec3f, dir: vec3f) -> BoxIntersection {
	let d_min = -orig / dir;
	let d_max = (vec3f(model.size.xyz) - orig) / dir;
	let d1 = min(d_min, d_max);
	let d2 = max(d_min, d_max);
	let t_min = max(max(max(d1.x, d1.y), d1.z), 0.0);
	let t_max = min(min(d2.x, d2.y), d2.z);
	let hit = t_min < t_max && t_max > 0.0;
	return BoxIntersection(hit, t_min, t_max);
}

struct CastResult {
	voxel: u32,
	hit: bool,
	t: f32,
	pos: vec3u,
	norm: vec3f,
}

fn ray_cast(orig: vec3f, dir: vec3f) -> CastResult {
	let half_size = vec3f(model.size.xyz) / 2.0;

	let intersection = box_intersection(orig, dir);
	if(!intersection.hit) {
		return CastResult(0, false, 0.0, vec3u(orig), dir);
	}

	let start = orig + dir * intersection.t_min;
	let step = vec3i(sign(dir));
	let t_delta = abs(1 / dir);

	var pos = clamp(vec3i(floor(start)), vec3i(0), model.size.xyz - 1);
	var norm = vec3f(0.0);
	var hit = false;
	var voxel = 0u;
	var t = intersection.t_min;

	var t_max = abs((vec3f(pos + vec3i(step > vec3i(0))) - start) / dir) + intersection.t_min;
	t_max = select(t_max, vec3f(intersection.t_max + 1.0), abs(dir) < vec3f(1.e-16));

	while(all(pos >= vec3i(0)) & all(pos < model.size.xyz)) {
		voxel = find_voxel(pos);

		if(voxel != 0) {
			hit = true;
			break;
		}

		if(t_max.x < t_max.y) {
			if(t_max.x < t_max.z) {
				pos.x += step.x;
				t = t_max.x;
				t_max.x += t_delta.x;
				norm = vec3f(1.0, 0.0, 0.0);
			} else {
				pos.z += step.z;
				t = t_max.z;
				t_max.z += t_delta.z;
				norm = vec3f(0.0, 0.0, 1.0);
			}
		} else {
			if(t_max.y < t_max.z) {
				pos.y += step.y;
				t = t_max.y;
				t_max.y += t_delta.y;
				norm = vec3f(0.0, 1.0, 0.0);
			} else {
				pos.z += step.z;
				t = t_max.z;
				t_max.z += t_delta.z;
				norm = vec3f(0.0, 0.0, 1.0);
			}
		}
	}

	let normf = select(norm * -vec3f(step), -dir, all(norm == vec3f(0.0)));

	return CastResult(voxel, hit, t, vec3u(pos), normf);
}

