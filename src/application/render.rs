use anyhow::Result;
use bytemuck::{Pod, Zeroable};
use wgpu::{Adapter, Buffer, BufferUsages, CompositeAlphaMode, Device, ExperimentalFeatures, Instance, MemoryHints, Queue, Surface, SurfaceCapabilities, SurfaceConfiguration, TextureFormat, TextureView, Trace};
use wgpu::util::DeviceExt;
use winit::dpi::PhysicalSize;
use winit::event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy};
use winit::window::Window;
use crate::platform;
use crate::platform::RcWindow;
use crate::utils::entropy::ENTROPY;
use crate::utils::math::{Mat4, Vec3};
use crate::utils::user_events::UserEvent;

pub const TIMING_QUERY_COUNT: u32 = 32;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Debug)]
pub struct Commons {
	pub view: Mat4,
	pub proj: Mat4,
	pub frame: u32,
	pub _pad1: u32,
	pub _pad2: u32,
	pub _pad3: u32,
}

#[derive(Debug)]
pub struct Render {
	pub window: RcWindow,
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
}

impl Render {
	pub async fn new(window: impl Into<RcWindow>) -> Result<Render> {
		let window = window.into();
		let size = window.inner_size();
		
		let instance = Instance::default();
		
		let surface = instance.create_surface(window.clone())?;
		
		let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions {
			power_preference: wgpu::PowerPreference::default(),
			force_fallback_adapter: false,
			compatible_surface: Some(&surface),
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
}

pub enum RenderState {
	Init(EventLoopProxy<UserEvent>),
	Initializing,
	Ready(Render),
}

impl RenderState {
	pub fn new(event_loop: &EventLoop<UserEvent>) -> RenderState {
		RenderState::Init(event_loop.create_proxy())
	}
	
	pub fn can_init(&mut self) -> bool {
		matches!(self, RenderState::Init(..))
	}
	
	pub fn init(&mut self, event_loop: &ActiveEventLoop) -> Result<()> {
		if !self.can_init() { return Ok(()) }
		
		let proxy = match std::mem::replace(self, RenderState::Initializing) {
			RenderState::Init(proxy) => proxy,
			_ => unreachable!(),
		};
		
		let win_attr =
			Window::default_attributes()
			       .with_title("Voxer")
			       .with_inner_size(PhysicalSize::new(1280, 720))
			       .with_transparent(true);
		
		let win_attr = platform::set_window_attributes(win_attr)?;
		
		let window = event_loop.create_window(win_attr)?;
		
		platform::spawn_future(async move {
			let graphics = Render::new(window).await.expect("Failed to create graphics");
			proxy.send_event(UserEvent::GraphicsReady(graphics)).expect("Failed to send graphics ready event");
		});
		
		Ok(())
	}
}
