use std::sync::Arc;
use anyhow::Result;
use bytemuck::{Pod, Zeroable};
use wgpu::{Adapter, Buffer, BufferUsages, CompositeAlphaMode, Device, Instance, Queue, Surface, SurfaceCapabilities, SurfaceConfiguration, TextureFormat, TextureView};
use wgpu::util::DeviceExt;
use winit::dpi::PhysicalSize;
use winit::window::Window;
use crate::utils::entropy::ENTROPY;
use crate::utils::math::{Mat4, Vec3};

pub const TIMING_QUERY_COUNT: u32 = 32;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct Commons {
	pub view: Mat4,
	pub proj: Mat4,
	pub frame: u32,
	pub _pad1: u32,
	pub _pad2: u32,
	pub _pad3: u32,
}

#[allow(unused)]
pub struct RenderContext {
	pub instance: Instance,
	pub surface: Surface<'static>,
	pub adapter: Adapter,
	pub device: Device,
	pub queue: Queue,
	pub caps: SurfaceCapabilities,
	pub swapchain_format: TextureFormat,
	pub entropy_tex: TextureView,
	pub commons_buf: Buffer,
	pub commons: Commons,
	pub config: SurfaceConfiguration,
}

impl RenderContext {
	pub async fn new(window: Arc<Window>) -> Result<Self> {
		let size = window.inner_size();
		
		let instance = Instance::default();
		
		let surface = instance.create_surface(window)?;
		
		let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions {
			power_preference: wgpu::PowerPreference::default(),
			force_fallback_adapter: false,
			compatible_surface: Some(&surface),
		}).await.expect("Unable to request an adapter");
		
		let required_features = wgpu::Features::PUSH_CONSTANTS;
		
		#[cfg(not(target_arch = "wasm32"))]
		let required_features = required_features
			| wgpu::Features::TIMESTAMP_QUERY
			| wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS
			| wgpu::Features::TIMESTAMP_QUERY_INSIDE_PASSES;
		
		let (device, queue) = adapter.request_device(
			&wgpu::DeviceDescriptor {
				label: Some("Main Device"),
				required_features,
				// Make sure we use the texture resolution limits from the adapter, so we can support images the size of the swapchain.
				required_limits: wgpu::Limits {
					max_push_constant_size: 64,
					max_buffer_size: 2 * 1024 * 1024 * 1024 - 1,
					max_storage_buffer_binding_size: 2 * 1024 * 1024 * 1024 - 1,
					..Default::default()
				},
			},
			None,
		).await?;
		
		let caps = surface.get_capabilities(&adapter);
		let swapchain_format = caps.formats[0];
		
		let mut config = surface.get_default_config(&adapter, size.width, size.height).unwrap();
		
		if caps.alpha_modes.contains(&CompositeAlphaMode::PreMultiplied) {
			config.alpha_mode = CompositeAlphaMode::PreMultiplied;
		}
		
		surface.configure(&device, &config);
		
		let entropy_tex = device.create_texture_with_data(&queue, &wgpu::TextureDescriptor {
			label: Some("Entropy Texture"),
			size: wgpu::Extent3d { width: 512, height: 512, depth_or_array_layers: 1 },
			mip_level_count: 1,
			sample_count: 1,
			dimension: wgpu::TextureDimension::D2,
			format: TextureFormat::Rgba8Unorm,
			usage: wgpu::TextureUsages::TEXTURE_BINDING,
			view_formats: &[TextureFormat::Rgba8Unorm],
		}, wgpu::util::TextureDataOrder::LayerMajor, ENTROPY);
		
		let entropy_tex = entropy_tex.create_view(&wgpu::TextureViewDescriptor { label: Some("Entropy Texture View"), ..Default::default() });
		
		let commons = Commons {
			proj: Mat4::identity(),
			view: Mat4::new_nonuniform_scaling(&Vec3::new(1.0, 1.0, 1.0)),
			frame: 0,
			..Commons::zeroed()
		};
		
		let commons_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
			label: Some("Commons Buffer"),
			contents: bytemuck::bytes_of(&commons),
			usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
		});
		
		Ok(RenderContext {
			instance,
			surface,
			adapter,
			device,
			queue,
			caps,
			swapchain_format,
			entropy_tex,
			commons_buf,
			commons,
			config,
		})
	}
	
	pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
		self.config.width = new_size.width.max(1);
		self.config.height = new_size.height.max(1);
		self.surface.configure(&self.device, &self.config);
	}
	
	pub fn update_commons(&self) {
		self.queue.write_buffer(&self.commons_buf, 0, bytemuck::bytes_of(&self.commons))
	}
}
