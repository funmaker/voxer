use anyhow::{Error, Result};
use bytemuck::Zeroable;
use nalgebra::{vector, Matrix};
use wgpu::{Buffer, BufferAddress, BufferDescriptor, BufferUsages, CommandEncoder, ComputePipeline, RenderPass, RenderPipeline};
use wgpu::util::{BufferInitDescriptor, DeviceExt};

mod model;

use crate::utils::math::{Mat4, Vec3, Vec4};
use crate::application::render::Render;
use crate::shaders::hashmap::commons as hashmap;
use crate::shaders::graphics::world;
use crate::shaders::compute::diffuse;
use crate::shaders::model::Model;
use crate::shaders::stats::Stats;

const HASHMAP_SIZE: BufferAddress = size_of::<u32>() as BufferAddress * hashmap::ENTRY_SIZE as BufferAddress * 100_0000;
const DIFFUSE_BLOCK_SIZE: u32 = 16;

impl world::Vertex {
	const fn new(x: f32, y: f32, z: f32) -> Self {
		world::Vertex {
			position: Vec4::new(x, y, z, 1.0),
		}
	}
}

pub struct World {
	pub model: Box<Model>,
	pub size: Vec3,
	pub center: Vec3,
	pub stats: Stats,
	vertex_buf: Buffer,
	stats_buf: Buffer,
	stats_buf_readback: Buffer,
	hashmaps: [Buffer; 2],
	bg_world_commons: world::bind_groups::BindGroup0,
	bg_world_hashmap: [world::bind_groups::BindGroup1; 2],
	bg_world_model: world::bind_groups::BindGroup2,
	bg_diffuse_commons: diffuse::bind_groups::BindGroup0,
	bg_diffuse_hashmap: [diffuse::bind_groups::BindGroup1; 2],
	bg_diffuse_model: diffuse::bind_groups::BindGroup2,
	world_pipeline: RenderPipeline,
	diffuse_pipeline: ComputePipeline,
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
		
		let stats_buf = render.device.create_buffer(&BufferDescriptor {
			label: Some("World Stats Buffer"),
			usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC | BufferUsages::COPY_DST,
			size: size_of::<Stats>() as BufferAddress,
			mapped_at_creation: false,
		});
		
		let stats_buf_readback = render.device.create_buffer(&BufferDescriptor {
			label: Some("World Stats Buffer Readback"),
			usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
			size: size_of::<Stats>() as BufferAddress,
			mapped_at_creation: false,
		});
		
		let hashmaps = [
			render.device.create_buffer(&BufferDescriptor {
				label: Some("World Hashmap 1"),
				size: HASHMAP_SIZE,
				usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
				mapped_at_creation: false,
			}),
			render.device.create_buffer(&BufferDescriptor {
				label: Some("World Hashmap 2"),
				size: HASHMAP_SIZE,
				usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
				mapped_at_creation: false,
			}),
		];
		
		let bg_world_commons = world::bind_groups::BindGroup0::from_bindings(&render.device, world::bind_groups::BindGroupLayout0 {
			commons: render.commons_buf.as_entire_buffer_binding(),
			stats: stats_buf.as_entire_buffer_binding(),
		});
		
		let bg_world_hashmap = [
			world::bind_groups::BindGroup1::from_bindings(&render.device, world::bind_groups::BindGroupLayout1 {
				hashmap: hashmaps[0].as_entire_buffer_binding(),
			}),
			world::bind_groups::BindGroup1::from_bindings(&render.device, world::bind_groups::BindGroupLayout1 {
				hashmap: hashmaps[1].as_entire_buffer_binding(),
			}),
		];
		
		let bg_world_model = world::bind_groups::BindGroup2::from_bindings(&render.device, world::bind_groups::BindGroupLayout2 {
			model: voxel_head_buf.as_entire_buffer_binding(),
		});
		
		let bg_diffuse_commons = diffuse::bind_groups::BindGroup0::from_bindings(&render.device, diffuse::bind_groups::BindGroupLayout0 {
			commons: render.commons_buf.as_entire_buffer_binding(),
			entropy_tex: &render.entropy_tex,
			stats: stats_buf.as_entire_buffer_binding(),
		});
		
		let bg_diffuse_hashmap = [
			diffuse::bind_groups::BindGroup1::from_bindings(&render.device, diffuse::bind_groups::BindGroupLayout1 {
				hashmap: hashmaps[0].as_entire_buffer_binding(),
			}),
			diffuse::bind_groups::BindGroup1::from_bindings(&render.device, diffuse::bind_groups::BindGroupLayout1 {
				hashmap: hashmaps[1].as_entire_buffer_binding(),
			}),
		];
		
		let bg_diffuse_model = diffuse::bind_groups::BindGroup2::from_bindings(&render.device, diffuse::bind_groups::BindGroupLayout2 {
			model: voxel_head_buf.as_entire_buffer_binding(),
		});
		
		let vertex_buf = render.device.create_buffer_init(&BufferInitDescriptor {
			label: Some("World Vertex Buffer"),
			contents: bytemuck::cast_slice(&[
				world::Vertex::new(-1.0, -1.0, 0.0),
				world::Vertex::new(1.0, -1.0, 0.0),
				world::Vertex::new(1.0, 1.0, 0.0),
				world::Vertex::new(-1.0, -1.0, 0.0),
				world::Vertex::new(1.0, 1.0, 0.0),
				world::Vertex::new(-1.0, 1.0, 0.0),
			]),
			usage: BufferUsages::VERTEX,
		});
		
		let world_pipeline = {
			let shader = world::create_shader_module(&render.device);
			let pipeline_layout = world::create_pipeline_layout(&render.device);
			
			render.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
				label: Some("World Pipeline"),
				layout: Some(&pipeline_layout),
				vertex: crate::shaders::vertex_state(&shader, &world::vs_main_entry(wgpu::VertexStepMode::Vertex)),
				fragment: Some(crate::shaders::fragment_state(&shader, &world::fs_main_entry([Some(render.swapchain_format.into())]))),
				primitive: wgpu::PrimitiveState::default(),
				depth_stencil: None,
				multisample: wgpu::MultisampleState::default(),
				multiview_mask: None,
				cache: None,
			})
		};
		
		let diffuse_pipeline = diffuse::compute::create_main_pipeline(&render.device, &diffuse::OverrideConstants {
			block_size: Some(DIFFUSE_BLOCK_SIZE),
		});
		
		Ok(World {
			size: Vec3::new(model.size.x as f32, model.size.y as f32, model.size.z as f32),
			center,
			model,
			stats: Stats::zeroed(),
			vertex_buf,
			stats_buf,
			stats_buf_readback,
			hashmaps,
			bg_world_commons,
			bg_world_hashmap,
			bg_world_model,
			bg_diffuse_commons,
			bg_diffuse_hashmap,
			bg_diffuse_model,
			world_pipeline,
			diffuse_pipeline,
		})
	}
	
	pub fn before_render(&mut self, encoder: &mut CommandEncoder, render: &Render) {
		self.hashmaps.swap(0, 1);
		// self.bg_world_hashmap.swap(0, 1);
		// self.bg_diffuse_hashmap.swap(0, 1);
		
		if let Ok(map) = self.stats_buf_readback.get_mapped_range(..) {
			self.stats = *bytemuck::from_bytes(&map);
			drop(map);
			self.stats_buf_readback.unmap();
		}
		
		// encoder.clear_buffer(&self.hashmaps[0], 0, None);
		encoder.clear_buffer(&self.stats_buf, 0, None);
		
		let width = render.surface_config.width.div_ceil(DIFFUSE_BLOCK_SIZE) * DIFFUSE_BLOCK_SIZE;
		let height = render.surface_config.height.div_ceil(DIFFUSE_BLOCK_SIZE) * DIFFUSE_BLOCK_SIZE;
		
		let mvp =
			Mat4::new_translation(&self.center)
				* render.pov.to_homogeneous()
				* Mat4::new_nonuniform_scaling(&vector!(
					(render.fov / 2.0).tan(),
					(render.fov / 2.0).tan() / render.aspect_ratio(),
					1.0,
				))
				* Mat4::new(
					2.0 / width as f32, 0.0,                 1.0, 0.0,
					0.0,                2.0 / height as f32, 1.0, 0.0,
					0.0,                0.0,                 1.0, 0.0,
					0.0,                0.0,                 0.0, 1.0,
				);
		
		let mut compute_pass = encoder.begin_compute_pass(&&wgpu::ComputePassDescriptor::default());
		compute_pass.set_pipeline(&self.diffuse_pipeline);
		compute_pass.set_immediates(0, bytemuck::bytes_of(&diffuse::Pc { mvp }));
		diffuse::set_bind_groups(&mut compute_pass, &self.bg_diffuse_commons, &self.bg_diffuse_hashmap[0], &self.bg_diffuse_model);
		compute_pass.dispatch_workgroups(width / DIFFUSE_BLOCK_SIZE, height / DIFFUSE_BLOCK_SIZE, 1);
		drop(compute_pass);
	}
	
	pub fn render<'s>(&'s mut self, rpass: &mut RenderPass<'s>, render: &Render) {
		let mvp =
			Mat4::new_translation(&self.center)
			* render.pov.to_homogeneous()
			* Matrix::new_nonuniform_scaling(&vector!((render.fov / 2.0).tan(), (render.fov / 2.0).tan() / render.aspect_ratio(), 1.0));
		
		rpass.push_debug_group("Prepare world data for draw.");
		rpass.set_pipeline(&self.world_pipeline);
		rpass.set_vertex_buffer(0, self.vertex_buf.slice(..));
		world::set_bind_groups(rpass, &self.bg_world_commons, &self.bg_world_hashmap[0], &self.bg_world_model);
		rpass.set_immediates(0, bytemuck::bytes_of(&world::Pc { mvp }));
		rpass.pop_debug_group();
		rpass.insert_debug_marker("Draw world!");
		rpass.draw(0..6, 0..1);
	}
	
	pub fn after_render(&mut self, encoder: &mut CommandEncoder) {
		encoder.copy_buffer_to_buffer(
			&self.stats_buf,
			0,
			&self.stats_buf_readback,
			0,
			self.stats_buf.size(),
		);
		encoder.map_buffer_on_submit(&self.stats_buf_readback, wgpu::MapMode::Read, .., |result| {
			result.expect("Failed to map stats buffer");
		});
	}
	
	pub fn gui(&mut self, ui: &mut egui::Ui) {
		let max_entries = self.hashmaps[0].size() / hashmap::ENTRY_SIZE as u64;
		ui.label(format!("Diffuse map: {:.0}% ({} / {})", self.stats.entries as f32 / max_entries as f32 * 100.0, self.stats.entries, max_entries));
		ui.label(format!("Collisions: {}", self.stats.collisions));
	}
}
