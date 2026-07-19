use std::borrow::Cow;
use std::mem::size_of;
use anyhow::{Error, Result};
use bytemuck::{Pod, Zeroable};
use wgpu::{BindGroup, Buffer, BufferUsages, PipelineLayoutDescriptor, RenderPass, RenderPipeline};
use wgpu::util::{BufferInitDescriptor, DeviceExt};

mod model;

use crate::utils::math::{Mat4, Vec3};
use crate::application::render_context::{Commons, RenderContext};
use crate::application::shaders;
use model::Model;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex {
	pos: [f32; 4],
}

impl Vertex {
	const fn new(x: f32, y: f32, z: f32) -> Self {
		Vertex {
			pos: [x, y, z, 1.0],
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
	bind_group: BindGroup,
}

impl World {
	pub fn new(model_path: &str, render: &RenderContext) -> Result<Self> {
		let vox_data = dot_vox::load(model_path).map_err(Error::msg)?;
		let (model, center) = Model::new(&vox_data);
		
		let bind_group_layout = render.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
			label: Some("World Bind Group Layout"),
			entries: &[
				wgpu::BindGroupLayoutEntry {
					binding: 0,
					visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
					ty: wgpu::BindingType::Buffer {
						ty: wgpu::BufferBindingType::Uniform,
						has_dynamic_offset: false,
						min_binding_size: wgpu::BufferSize::new(size_of::<Commons>() as u64),
					},
					count: None,
				},
				wgpu::BindGroupLayoutEntry {
					binding: 1,
					visibility: wgpu::ShaderStages::FRAGMENT,
					ty: wgpu::BindingType::Buffer {
						ty: wgpu::BufferBindingType::Storage { read_only: true },
						has_dynamic_offset: false,
						min_binding_size: wgpu::BufferSize::new(Model::min_binding_size() as u64),
					},
					count: None,
				},
				wgpu::BindGroupLayoutEntry {
					binding: 2,
					visibility: wgpu::ShaderStages::FRAGMENT,
					ty: wgpu::BindingType::Texture {
						sample_type: wgpu::TextureSampleType::Float { filterable: false },
						view_dimension: wgpu::TextureViewDimension::D2,
						multisampled: false,
					},
					count: None,
				},
			],
		});
		
		let voxel_head_buf = render.device.create_buffer_init(&BufferInitDescriptor {
			label: Some("World Voxel Buffer"),
			contents: model.as_bytes(),
			usage: BufferUsages::STORAGE,
		});
		
		let bind_group = render.device.create_bind_group(&wgpu::BindGroupDescriptor {
			label: Some("World Bind Group"),
			layout: &bind_group_layout,
			entries: &[
				wgpu::BindGroupEntry {
					binding: 0,
					resource: render.commons_buf.as_entire_binding(),
				},
				wgpu::BindGroupEntry {
					binding: 1,
					resource: voxel_head_buf.as_entire_binding(),
				},
				wgpu::BindGroupEntry {
					binding: 2,
					resource: wgpu::BindingResource::TextureView(&render.entropy_tex),
				},
			],
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
		
		let pipeline_layout = render.device.create_pipeline_layout(&PipelineLayoutDescriptor {
			label: Some("World Pipeline Layout"),
			bind_group_layouts: &[&bind_group_layout],
			push_constant_ranges: &[wgpu::PushConstantRange {
				stages: wgpu::ShaderStages::VERTEX_FRAGMENT,
				range: 0..(size_of::<Pc>() as u32),
			}],
		});
		
		let shader = render.device.create_shader_module(wgpu::ShaderModuleDescriptor {
			label: Some("World Shader"),
			source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(shaders::world::SOURCE)),
		});
		
		let vertex_buffers = [wgpu::VertexBufferLayout {
			array_stride: size_of::<Vertex>() as wgpu::BufferAddress,
			step_mode: wgpu::VertexStepMode::Vertex,
			attributes: &[
				wgpu::VertexAttribute {
					format: wgpu::VertexFormat::Float32x4,
					offset: 0,
					shader_location: 0,
				},
			],
		}];
		
		let pipeline = render.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
			label: Some("World Pipeline"),
			layout: Some(&pipeline_layout),
			vertex: wgpu::VertexState {
				module: &shader,
				entry_point: "vs_main",
				buffers: &vertex_buffers,
				compilation_options: Default::default(),
			},
			fragment: Some(wgpu::FragmentState {
				module: &shader,
				entry_point: "fs_main",
				compilation_options: Default::default(),
				targets: &[Some(render.swapchain_format.into())],
			}),
			primitive: wgpu::PrimitiveState::default(),
			depth_stencil: None,
			multisample: wgpu::MultisampleState::default(),
			multiview: None,
		});
		
		Ok(World {
			size: Vec3::new(model.width as f32, model.height as f32, model.depth as f32),
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
		rpass.set_bind_group(0, &self.bind_group, &[]);
		rpass.set_push_constants(wgpu::ShaderStages::VERTEX_FRAGMENT, 0, bytemuck::bytes_of(&Pc { model }));
		rpass.pop_debug_group();
		rpass.insert_debug_marker("Draw world!");
		rpass.draw(0..6, 0..1);
	}
}
