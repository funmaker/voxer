use std::alloc::Layout;
use std::{alloc, ptr, slice};
use std::mem::size_of_val_raw;
use std::ptr::Pointee;
use bytemuck::{Pod, Zeroable};
use dot_vox::SceneNode;
use nalgebra::{point, Point, vector};

use crate::utils::math::Vec3;

#[repr(C, align(4))]
pub struct Model {
	pub width: u32,
	pub height: u32,
	pub depth: u32,
	_pad: u32,
	pub palette: [Material; 256],
	pub voxels: [u8],
}

impl Model {
	pub fn new(vox: &dot_vox::DotVoxData) -> (Box<Self>, Vec3) {
		let scene = parse_scene(vox);
		
		let mut aabb_min = vector!(i32::MAX, i32::MAX, i32::MAX);
		let mut aabb_max = vector!(i32::MIN, i32::MIN, i32::MIN);
		let apply = |point: nalgebra::Point3<i32>, transform: VoxTransform| nalgebra::Point3::from_homogeneous(transform * point.to_homogeneous()).unwrap();
		
		for (transform, model_id) in scene.iter().copied() {
			let model = &vox.models[model_id];
			let a = apply(Point::origin(), transform);
			let b = apply(point![model.size.x as i32 - 1, model.size.y as i32 - 1, model.size.z as i32 - 1], transform);
			
			aabb_min.x = aabb_min.x.min(a.x).min(b.x);
			aabb_min.y = aabb_min.y.min(a.y).min(b.y);
			aabb_min.z = aabb_min.z.min(a.z).min(b.z);
			aabb_max.x = aabb_max.x.max(a.x).max(b.x);
			aabb_max.y = aabb_max.y.max(a.y).max(b.y);
			aabb_max.z = aabb_max.z.max(a.z).max(b.z);
		}
		
		let width = (aabb_max.x - aabb_min.x + 1) as u32;
		let height = (aabb_max.y - aabb_min.y + 1) as u32;
		let depth = (aabb_max.z - aabb_min.z + 1) as u32;
		
		let mut this = Model::new_empty(width, height, depth);
		
		for (n, entry) in vox.palette.iter().copied().enumerate().take(255) {
			let mut palette_mat: Material = entry.into();
			
			if let Some(material) = vox.materials.get(n) {
				let get_prop = |name: &str, default: f32| material.properties.get(name)
				                                                             .and_then(|s| s.parse().ok())
				                                                             .unwrap_or(default);
				
				match material.properties.get("_type").map(|x| &**x) {
					Some("_metal") => {
						palette_mat.rough = get_prop("_rough", 0.0);
						palette_mat.ior = get_prop("_ior", 0.0);
						palette_mat.metal = get_prop("_metal", 0.0);
					},
					Some("_emit") => {
						palette_mat.lum = get_prop("_emit", 0.0) * 10.0_f32.powf(get_prop("_flux", 0.0))
					},
					_ => {}
				}
			}
			
			this.palette[n + 1] = palette_mat;
		}
		
		for (transform, model_id) in scene.iter().copied() {
			let model = &vox.models[model_id];
			
			for voxel in model.voxels.iter() {
				let position = apply(point![voxel.x as i32, voxel.y as i32, voxel.z as i32], transform) - aabb_min;
				
				assert!(position.x >= 0);
				assert!(position.y >= 0);
				assert!(position.z >= 0);
				assert!(position.x < this.width as i32);
				assert!(position.y < this.height as i32);
				assert!(position.z < this.depth as i32);
				
				this.voxels[
					position.x as usize
					+ position.y as usize * this.width as usize
					+ position.z as usize * this.width as usize * this.height as usize
				] = voxel.i + 1;
			}
		}
		
		let center = (-aabb_min).cast();
		
		(this, center)
	}
	
	pub fn as_bytes(&self) -> &[u8] {
		assert!(self.palette.len() % align_of_val(self) == 0);
		
		// SAFETY: self must have no padding
		unsafe {
			slice::from_raw_parts(self as *const _ as *const u8, size_of_val(self))
		}
	}
	
	pub fn min_binding_size() -> usize {
		// it seems that wgpu incorrectly rounds up min buffer binding size to struct alignment for some reason???
		// https://www.w3.org/TR/webgpu/#minimum-buffer-binding-size
		// TODO: wtf
		unsafe { size_of_val_raw(ptr::from_raw_parts::<Self>(ptr::null::<()>(), 16)) }
	}
	
	pub fn size(&self) -> Vec3 {
		Vec3::new(self.width as f32, self.height as f32, self.depth as f32)
	}
	
	fn new_empty(width: u32, height: u32, depth: u32) -> Box<Self> {
		let voxel_count: usize = (width as usize * height as usize * depth as usize).div_ceil(4) * 4;
		
		// SAFETY: Statically known part of Self must fit within isize::MAX bytes (very likely)
		// Size of entire allocation must be no larger isize::MAX
		// Self must be made of plain data that can be zeroed
		unsafe {
			let header_size = size_of_val_raw(ptr::from_raw_parts::<Self>(ptr::null::<()>(), 0));
			assert!(voxel_count < isize::MAX as usize - header_size);
			
			let metadata: <Self as Pointee>::Metadata = voxel_count;
			let ptr: *const Self = ptr::from_raw_parts(ptr::null::<()>(), metadata);
			let layout = Layout::for_value_raw(ptr);
			let this = alloc::alloc_zeroed(layout);
			if this.is_null() {
				alloc::handle_alloc_error(layout);
			}
			
			let mut this: Box<Self> = Box::from_raw(ptr::from_raw_parts_mut(this, metadata));
			this.width = width;
			this.height = height;
			this.depth = depth;
			this.palette = [Material::TRANSPARENT; _];
			
			this
		}
	}
}

type VoxTransform = nalgebra::Matrix4<i32>;

const R_X: nalgebra::RowVector4<i32> = nalgebra::RowVector4::new(1, 0, 0, 0);
const R_Y: nalgebra::RowVector4<i32> = nalgebra::RowVector4::new(0, 1, 0, 0);
const R_Z: nalgebra::RowVector4<i32> = nalgebra::RowVector4::new(0, 0, 1, 0);
const R_W: nalgebra::RowVector4<i32> = nalgebra::RowVector4::new(0, 0, 0, 1);

fn parse_scene(vox: &dot_vox::DotVoxData) -> Vec<(VoxTransform, usize)> {
	let mut scene = vec![];
	
	if !vox.scenes.is_empty() {
		parse_scene_impl(vox, 0, VoxTransform::from_rows(&[R_X, R_Z, -R_Y, R_W]), &mut scene);
	}
	
	scene
}

fn parse_scene_impl(vox: &dot_vox::DotVoxData, node: u32, mut transform: VoxTransform, scene: &mut Vec<(VoxTransform, usize)>) {
	match &vox.scenes[node as usize] {
		SceneNode::Transform { frames, child, .. } => {
			for frame in frames {
				if let Some(translate) = frame.attributes.get("_t") {
					let [x, y, z] = translate.split(" ")
					                         .map(|part| part.parse())
					                         .collect::<Result<Vec<_>, _>>()
					                         .expect("Can't parse vox scene node _t, expected number")
					                         .try_into()
					                         .expect("Can't parse vox scene node _t, expected 3 numbers");
					
					transform = transform * VoxTransform::new_translation(&vector![x, y, z]);
				}
				if let Some(rotate) = frame.attributes.get("_r") {
					let r: u8 = rotate.parse().expect("Can't parse vox scene node _r, expected number");
					
					let mut rot = match r & 0b1111 {
						0b0100 => VoxTransform::from_rows(&[R_X, R_Y, R_Z, R_W]),
						0b0001 => VoxTransform::from_rows(&[R_Y, R_X, R_Z, R_W]),
						0b1000 => VoxTransform::from_rows(&[R_X, R_Z, R_Y, R_W]),
						0b0010 => VoxTransform::from_rows(&[R_Z, R_Y, R_X, R_W]),
						0b1001 => VoxTransform::from_rows(&[R_Y, R_Z, R_X, R_W]),
						0b0110 => VoxTransform::from_rows(&[R_Z, R_Y, R_X, R_W]),
						m => panic!("Can't parse vox scene node _r, invalid matrix int {m:b}({rotate} - {r:b})")
					};
					
					if r & 0b0010000 != 0 { rot = VoxTransform::from_diagonal(&vector![-1, 1, 1, 1]) * rot; }
					if r & 0b0100000 != 0 { rot = VoxTransform::from_diagonal(&vector![1, -1, 1, 1]) * rot; }
					if r & 0b1000000 != 0 { rot = VoxTransform::from_diagonal(&vector![1, 1, -1, 1]) * rot; }
					
					transform = transform * rot;
				}
			}
			
			parse_scene_impl(vox, *child, transform, scene);
		},
		SceneNode::Group { children, .. } => {
			for &child in children {
				parse_scene_impl(vox, child, transform, scene);
			}
		},
		SceneNode::Shape { models, .. } => {
			for shape_model in models {
				let model = &vox.models[shape_model.model_id as usize];
				
				// TODO: THERE MUST BE A BETTER WAY
				transform = transform * VoxTransform::new_translation(&vector![
					model.size.x.div_floor(2) as i32 * -1 + if transform.column(0).rows(0, 3).min() < 0 { 1 } else { 0 },
					model.size.y.div_floor(2) as i32 * -1 + if transform.column(1).rows(0, 3).min() < 0 { 1 } else { 0 },
					model.size.z.div_floor(2) as i32 * -1 + if transform.column(2).rows(0, 3).min() < 0 { 1 } else { 0 },
				]);
				
				scene.push((transform, shape_model.model_id as usize));
			}
		}
	}
}



#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct Material {
	red: f32,
	green: f32,
	blue: f32,
	alpha: f32,
	lum: f32,
	rough: f32,
	ior: f32,
	metal: f32,
}

impl Material {
	const fn new(red: f32, green: f32, blue: f32, alpha: f32) -> Self {
		Material {
			red,
			green,
			blue,
			alpha,
			lum: 0.0,
			rough: 1.0,
			ior: 0.0,
			metal: 0.0,
		}
	}
	
	const TRANSPARENT: Self = Self::new(0.0, 0.0, 0.0, 0.0);
}

impl From<dot_vox::Color> for Material {
	fn from(value: dot_vox::Color) -> Material {
		Material::new(
			value.r as f32 / 255.0,
			value.g as f32 / 255.0,
			value.b as f32 / 255.0,
			value.a as f32 / 255.0
		)
	}
}
