struct Commons {
  view: mat4x4f,
  proj: mat4x4f,
  frame: u32,
}
@group(0) @binding(0) var<uniform> commons: Commons;

struct Material {
  col: vec4f,
  lum: f32,
  rough: f32,
  ior: f32,
  metal: f32,
}

struct Model {
  size: vec4i,
  palette: array<Material, 256>,
  voxels: array<u32>,
}
@group(0) @binding(1) var<storage> model: Model;

@group(0) @binding(2) var entropy_tex: texture_2d<f32>;

struct Pc {
  model: mat4x4<f32>,
}
var<push_constant> pc: Pc;

const pi = radians(180.0);
const tau = radians(360.0);

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) orig: vec4f,
    @location(1) targ: vec4f,
    @location(2) pos2: vec4f,
};

@vertex
fn vs_main(
  @location(0) position: vec4f,
) -> VertexOutput {
  var result: VertexOutput;
  
  result.position = position;
  result.pos2 = position;
  result.orig = pc.model * commons.view * vec4(0.0, 0.0, 0.0, 1.0);
  result.targ = pc.model * commons.view * vec4(position.x, position.y, -1.0, 1.0);
  
  return result;
}

@fragment
fn fs_main(vertex: VertexOutput) -> @location(0) vec4f {
  var orig = vertex.orig.xyz;
  var dir = normalize(vertex.targ.xyz - vertex.orig.xyz);
  var color = vec3f(0.0, 0.0, 0.0);
  var mask = vec3f(1.0);
  var dist = 0.0;
  
  for (var i = 0u; i < 3; i++) {
    let hit = ray_cast(orig, dir);
    
    if(!hit.hit) {
      break;
    }
    
    dist += hit.t;
    let hit_mat = model.palette[hit.voxel];
    let light = 1.0 / (1.0 + dist / 4.0);
    
    if(hit_mat.lum > 0.0) {
      color += mask * hit_mat.col.rgb * hit_mat.lum * light;
      break;
    }
    
    mask *= hit_mat.col.rgb;
    
    if(all(mask <= vec3f(1.0 / 128.0))) {
      break;
    }
    
    let random = get_random(vec3i(i32(vertex.position.x), i32(vertex.position.y), i32(commons.frame + i)));
    
    orig += dir * (hit.t - 0.01);
    
    let yaw = random.x * tau;
    let pitch = pi / 2 - acos(random.y);
    let up_guess = select(vec3f(0.0, 1.0, 0.0), vec3f(1.0, 0.0, 0.0), abs(hit.norm.y) >= 0.9);
    let right = normalize(cross(hit.norm, up_guess));
    let front = normalize(cross(hit.norm, right));
    
    dir = hit.norm * sin(pitch) + right * cos(pitch) * cos(yaw) + front * cos(pitch) * sin(yaw);
  }
  
  return vec4f(color, 1.0);
}

fn get_random(coords: vec3i) -> vec2f {
  let dims = vec2i(textureDimensions(entropy_tex));
  let x = (coords.x + coords.z / dims.y) % dims.x;
  let y = (coords.y + coords.z) % dims.y;
  return textureLoad(entropy_tex, vec2i(x, y), 0).xy;
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
  norm: vec3f,
}

fn ray_cast(orig: vec3f, dir: vec3f) -> CastResult {
  let half_size = vec3f(model.size.xyz) / 2.0;
  
  let intersection = box_intersection(orig, dir);
  if(!intersection.hit) {
    return CastResult(0, false, 0.0, dir);
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
  
  return CastResult(voxel, hit, t, normf);
}
