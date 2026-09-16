use std::sync::Arc;
use anyhow::Result;
use wgpu::{Adapter, Buffer, BufferUsages, CompositeAlphaMode, Device, ExperimentalFeatures, Instance, MemoryHints, Queue, Surface, SurfaceCapabilities, SurfaceConfiguration, TextureFormat, TextureView, Trace};
use wgpu::util::DeviceExt;
use winit::dpi::PhysicalSize;
use winit::window::Window;
use crate::platform;
use crate::shaders::commons::Commons;
use crate::utils::entropy::ENTROPY;
use crate::utils::math;
use crate::utils::math::{Isometry3, Vec3};

pub const TIMING_QUERY_COUNT: u32 = 32;

#[derive(Debug)]
pub struct Render {
	pub window: Arc<Window>,
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
	pub surface_config: SurfaceConfiguration,
	pub pov: Isometry3,
	pub pov_rot: (f32, f32, f32),
	pub fov: f32,
}

impl Render {
	pub async fn new(window: impl Into<Arc<Window>>) -> Result<Render> {
		let window = window.into();
		let size = window.inner_size();
		
		let instance = Instance::default();
		
		let surface = instance.create_surface(window.clone())?;
		
		let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions {
			power_preference: wgpu::PowerPreference::default(),
			force_fallback_adapter: false,
			compatible_surface: Some(&surface),
			apply_limit_buckets: false,
		}).await?;
		
		let required_features = wgpu::Features::IMMEDIATES;
		let required_features = platform::set_wgpu_features(required_features);
		
		let (device, queue) = adapter.request_device(
			&wgpu::DeviceDescriptor {
				label: Some("Main Device"),
				required_features,
				experimental_features: ExperimentalFeatures::disabled(),
				memory_hints: MemoryHints::Performance,
				trace: Trace::Off,
				// Make sure we use the texture resolution limits from the adapter, so we can support images the size of the swapchain.
				// TODO: ?
				required_limits: wgpu::Limits {
					max_immediate_size: 64,
					max_buffer_size: 1024 * 1024 * 1024 - 1,
					max_storage_buffer_binding_size: 1024 * 1024 * 1024 - 1,
					..Default::default()
				},
			}
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
			frame: 0,
		};
		
		let commons_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
			label: Some("Commons Buffer"),
			contents: bytemuck::bytes_of(&commons),
			usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
		});
		
		let pov_rot = (0.0, 0.0, 0.0);
		let pov = Isometry3::from_parts(Vec3::new(0.0, 64.0, 0.0).into(), math::from_euler(pov_rot.0, pov_rot.1, pov_rot.2));
		
		Ok(Render {
			window,
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
			surface_config: config,
			pov,
			pov_rot,
			fov: math::to_radians(90.0),
		})
	}
	
	pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
		self.surface_config.width = new_size.width.max(1);
		self.surface_config.height = new_size.height.max(1);
		self.surface.configure(&self.device, &self.surface_config);
	}
	
	pub fn update_commons(&self) {
		self.queue.write_buffer(&self.commons_buf, 0, bytemuck::bytes_of(&self.commons))
	}
	
	pub fn request_redraw(&self) {
		self.window.request_redraw();
	}
	
	pub fn aspect_ratio(&self) -> f32 {
		self.surface_config.width as f32 / self.surface_config.height as f32
	}
}
