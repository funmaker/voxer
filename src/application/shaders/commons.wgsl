
struct Commons {
  view: mat4x4f,
  proj: mat4x4f,
  frame: u32,
}

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

const pi = radians(180.0);
const tau = radians(360.0);
