#![allow(dead_code)]

pub use std::f32::consts::PI;

pub type Vec2 = nalgebra::Vector2<f32>;
pub type Vec3 = nalgebra::Vector3<f32>;
pub type Vec4 = nalgebra::Vector4<f32>;

pub type IVec2 = nalgebra::Vector2<i32>;
pub type IVec3 = nalgebra::Vector3<i32>;
pub type IVec4 = nalgebra::Vector4<i32>;

pub type Point2 = nalgebra::Point2<f32>;
pub type Point3 = nalgebra::Point3<f32>;
pub type Point4 = nalgebra::Point4<f32>;

pub type Rot3 = nalgebra::UnitQuaternion<f32>;
pub type Translation3 = nalgebra::Translation3<f32>;
pub type Isometry3 = nalgebra::Isometry3<f32>;
pub type Similarity3 = nalgebra::Similarity3<f32>;
pub type Perspective3 = nalgebra::Perspective3<f32>;

pub type Rot2 = nalgebra::UnitComplex<f32>;
pub type Translation2 = nalgebra::Translation2<f32>;
pub type Isometry2 = nalgebra::Isometry2<f32>;
pub type Similarity2 = nalgebra::Similarity2<f32>;

pub type AMat3 = nalgebra::Affine2<f32>;
pub type AMat4 = nalgebra::Affine3<f32>;

pub type PMat3 = nalgebra::Projective2<f32>;
pub type PMat4 = nalgebra::Projective3<f32>;

pub type Mat2 = nalgebra::Matrix2<f32>;
pub type Mat3 = nalgebra::Matrix3<f32>;
pub type Mat4 = nalgebra::Matrix4<f32>;
pub type Mat3x4 = nalgebra::Matrix3x4<f32>;

pub fn face_towards_lossy(dir: Vec3) -> Rot3 {
	if dir.cross(&Vec3::y_axis()).magnitude_squared() <= f32::EPSILON {
		Rot3::face_towards(&-dir, &Vec3::z_axis())
	} else {
		Rot3::face_towards(&-dir, &Vec3::y_axis())
	}
}

pub fn face_upwards_lossy(dir: Vec3) -> Rot3 {
	if dir.cross(&-Vec3::y_axis()).magnitude_squared() < f32::EPSILON {
		Rot3::identity()
	} else {
		Rot3::face_towards(&dir.cross(&-Vec3::y_axis()).cross(&dir), &dir)
	}
}

// Using ZXY euler sequence
// Thanks for help kirsh168
pub fn to_euler(rot: Rot3) -> (f32, f32, f32) {
	let m11 = 2.0 * (rot.w * rot.w + rot.i * rot.i) - 1.0;
	// let m12 = 2.0 * (rot.i * rot.j - rot.w * rot.k);
	let m13 = 2.0 * (rot.i * rot.k + rot.w * rot.j);
	
	let m21 = 2.0 * (rot.i * rot.j + rot.w * rot.k);
	let m22 = 2.0 * (rot.w * rot.w + rot.j * rot.j) - 1.0;
	let m23 = 2.0 * (rot.j * rot.k - rot.w * rot.i);
	
	let m31 = 2.0 * (rot.i * rot.k - rot.w * rot.j);
	// let m32 = 2.0 * (rot.j * rot.k + rot.w * rot.i);
	let m33 = 2.0 * (rot.w * rot.w + rot.k * rot.k) - 1.0;
	
	let pitch = -m23.clamp(-1.0, 1.0).asin();
	let gimbal_lock = pitch.abs() > PI / 2.0 - 0.001;
	
	let yaw = if gimbal_lock {
		f32::atan2(-m31, m11)
	} else {
		f32::atan2(m13, m33)
	};
	
	let xy_proj = m21 / pitch.cos();
	let roll = if gimbal_lock {
		0.0
	} else if m22 < 0.0 {
		PI.copysign(m21) - xy_proj.clamp(-1.0, 1.0).asin() // Upside down
	} else {
		xy_proj.clamp(-1.0, 1.0).asin()
	};
	
	(pitch, yaw, roll)
}

// Using ZXY euler sequence
pub fn from_euler(pitch: f32, yaw: f32, roll: f32) -> Rot3 {
	let x = Rot3::from_axis_angle(&Vec3::x_axis(), pitch);
	let y = Rot3::from_axis_angle(&Vec3::y_axis(), yaw);
	let z = Rot3::from_axis_angle(&Vec3::z_axis(), roll);
	
	y * x * z
}
