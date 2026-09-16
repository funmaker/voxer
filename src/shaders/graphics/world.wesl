import package::commons::{commons, get_random, tau, pi};
import package::stats::stats;
import package::hashmap::read as hashmap;
import package::model;

struct Pc {
	mvp: mat4x4<f32>,
}
var<immediate> pc: Pc;

struct Vertex {
	@location(0) position: vec4f,
};

struct VertexOutput {
		@builtin(position) position: vec4f,
		@location(0) orig: vec4f,
		@location(1) targ: vec4f,
};

@vertex
fn vs_main(vertex: Vertex) -> VertexOutput {
	var result: VertexOutput;
	
	result.position = vertex.position;
	result.orig = pc.mvp * vec4(0.0, 0.0, 0.0, 1.0);
	result.targ = pc.mvp * vec4(vertex.position.x, vertex.position.y, -1.0, 1.0);
	
	return result;
}

@fragment
fn fs_main(vertex: VertexOutput) -> @location(0) vec4f {
	atomicAdd(&stats.entries, 1);
	
	let frame = commons.frame;
	var color = vec3(0.0, 0.0, 0.0);
	
	var orig = vertex.orig.xyz;
	var dir = normalize(vertex.targ.xyz - vertex.orig.xyz);
	let hit = model::ray_cast(orig, dir);
	if !hit.hit {
		return vec4f(0.0, 0.0, 0.0, 0.0);
	}
	
	let hit_mat = model::model.palette[hit.voxel];
	if hit_mat.metal < 1.0 {
		let key = hashmap::getKey(hit.pos);
		let read_pos = hashmap::lookup(key);
		if(read_pos != hashmap::NOT_FOUND) {
			let read_value = vec4u(
				hashmap::hashmap[read_pos + 0],
				hashmap::hashmap[read_pos + 1],
				hashmap::hashmap[read_pos + 2],
				hashmap::hashmap[read_pos + 3]
			);
			
			color += vec3f(read_value.rgb) / f32(read_value.a) / 16.0 * (1.0 - hit_mat.metal);
		}
	}
	
	if hit_mat.metal > 0.0 {
		orig += dir * (hit.t - 0.01);
		dir = dir - hit.norm * (2.0 * dot(dir, hit.norm));
		let hit = model::ray_cast(orig, dir);
		
		if hit.hit {
			let hit_mat = model::model.palette[hit.voxel];
			let key = hashmap::getKey(hit.pos);
			let read_pos = hashmap::lookup(key);
			if(read_pos != hashmap::NOT_FOUND) {
				let read_value = vec4u(
					hashmap::hashmap[read_pos + 0],
					hashmap::hashmap[read_pos + 1],
					hashmap::hashmap[read_pos + 2],
					hashmap::hashmap[read_pos + 3]
				);
				
				color += vec3f(read_value.rgb) / f32(read_value.a) / 16.0 * (1.0 - hit_mat.metal);
			}
		}
	}
	
	return vec4f(color, 1.0);
}
