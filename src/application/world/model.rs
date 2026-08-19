use dot_vox::SceneNode;
use nalgebra::{point, Point, vector};

use crate::shaders::world::{Material, Model};
use crate::utils::math::{IVec4, Vec3, Vec4};

impl Model {
	pub fn load(vox: &dot_vox::DotVoxData) -> (Box<Model>, Vec3) {
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
		let voxel_count: usize = (width as usize * height as usize * depth as usize).div_ceil(4) * 4;
		
		let mut palette = [Material::TRANSPARENT; 256];
		let mut voxels = vec![0; voxel_count];
		
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
			
			palette[n + 1] = palette_mat;
		}
		
		for (transform, model_id) in scene.iter().copied() {
			let model = &vox.models[model_id];
			
			for voxel in model.voxels.iter() {
				let position = apply(point![voxel.x as i32, voxel.y as i32, voxel.z as i32], transform) - aabb_min;
				
				assert!(position.x >= 0);
				assert!(position.y >= 0);
				assert!(position.z >= 0);
				assert!(position.x < width as i32);
				assert!(position.y < height as i32);
				assert!(position.z < depth as i32);
				
				voxels[
					position.x as usize
					+ position.y as usize * width as usize
					+ position.z as usize * width as usize * height as usize
				] = voxel.i + 1;
			}
		}
		
		let center = (-aabb_min).cast();
		
		let model = Model {
			size: IVec4::new(width as i32, height as i32, depth as i32, 0),
			palette,
			voxels: bytemuck::cast_slice(&voxels),
		};
		
		(model.into(), center)
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

impl Material {
	const fn from_color(red: f32, green: f32, blue: f32, alpha: f32) -> Self {
		Material {
			col: Vec4::new(
				red,
				green,
				blue,
				alpha,
			),
			lum: 0.0,
			rough: 1.0,
			ior: 0.0,
			metal: 0.0,
		}
	}
	
	const TRANSPARENT: Self = Self::from_color(0.0, 0.0, 0.0, 0.0);
}

impl From<dot_vox::Color> for Material {
	fn from(value: dot_vox::Color) -> Material {
		Material::from_color(
			value.r as f32 / 255.0,
			value.g as f32 / 255.0,
			value.b as f32 / 255.0,
			value.a as f32 / 255.0
		)
	}
}
