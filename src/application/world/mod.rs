use anyhow::{Error, Result};
use bytemuck::{Pod, Zeroable};
use wgpu::{Buffer, BufferUsages, RenderPass, RenderPipeline};
use wgpu::util::{BufferInitDescriptor, DeviceExt};

mod model;

use crate::utils::math::{Mat4, Vec3, Vec4};
use crate::application::render::Render;
use crate::shaders::world::{ self as shader, Model, Vertex };

impl Vertex {
	const fn new(x: f32, y: f32, z: f32) -> Self {
		Vertex {
			position: Vec4::new(x, y, z, 1.0),
		}
	}
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Pc {
	model: Mat4,
}

pub struct World {
	pub model: Box<Model>,
	pub size: Vec3,
	pub center: Vec3,
	vertex_buf: Buffer,
	pipeline: RenderPipeline,
	bind_group: shader::bind_groups::BindGroup0,
}

impl World {
	pub fn new(model_path: &str, render: &Render) -> Result<Self> {
		let vox_data = dot_vox::load(model_path).map_err(Error::msg)?;
		let (model, center) = Model::load(&vox_data);
		
		let voxel_head_buf = render.device.create_buffer_init(&BufferInitDescriptor {
			label: Some("World Voxel Buffer"),
			contents: model.as_bytes(),
			usage: BufferUsages::STORAGE,
		});
		
		let bind_group = shader::bind_groups::BindGroup0::from_bindings(&render.device, shader::bind_groups::BindGroupLayout0 {
			commons: render.commons_buf.as_entire_buffer_binding(),
			model: voxel_head_buf.as_entire_buffer_binding(),
			entropy_tex: &render.entropy_tex,
		});
		
		let vertex_buf = render.device.create_buffer_init(&BufferInitDescriptor {
			label: Some("World Vertex Buffer"),
			contents: bytemuck::cast_slice(&[
				Vertex::new(-1.0, -1.0, 0.0),
				Vertex::new(1.0, -1.0, 0.0),
				Vertex::new(1.0, 1.0, 0.0),
				Vertex::new(-1.0, -1.0, 0.0),
				Vertex::new(1.0, 1.0, 0.0),
				Vertex::new(-1.0, 1.0, 0.0),
			]),
			usage: BufferUsages::VERTEX,
		});
		
		let shader = shader::create_shader_module(&render.device);
		let pipeline_layout = shader::create_pipeline_layout(&render.device);
		
		let pipeline = render.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
			label: Some("World Pipeline"),
			layout: Some(&pipeline_layout),
			vertex: crate::shaders::vertex_state(&shader, &shader::vs_main_entry(wgpu::VertexStepMode::Vertex)),
			fragment: Some(crate::shaders::fragment_state(&shader, &shader::fs_main_entry([Some(render.swapchain_format.into())]))),
			primitive: wgpu::PrimitiveState::default(),
			depth_stencil: None,
			multisample: wgpu::MultisampleState::default(),
			multiview_mask: None,
			cache: None,
		});
		
		Ok(World {
			size: Vec3::new(model.size.x as f32, model.size.y as f32, model.size.z as f32),
			center,
			model,
			vertex_buf,
			pipeline,
			bind_group,
		})
	}
	
	pub fn render<'s>(&'s mut self, rpass: &mut RenderPass<'s>) {
		let model = Mat4::new_translation(&self.center);
		
		rpass.push_debug_group("Prepare world data for draw.");
		rpass.set_pipeline(&self.pipeline);
		rpass.set_vertex_buffer(0, self.vertex_buf.slice(..));
		shader::set_bind_groups(rpass, &self.bind_group);
		rpass.set_immediates(0, bytemuck::bytes_of(&Pc { model }));
		rpass.pop_debug_group();
		rpass.insert_debug_marker("Draw world!");
		rpass.draw(0..6, 0..1);
	}
}
