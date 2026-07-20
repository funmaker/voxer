use std::time::Instant;
use winit::application::{ApplicationHandler};
use winit::dpi::PhysicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{WindowId};

pub mod render;
mod bench;
mod gpu_bench;
mod gui;
mod input;
mod shaders;
mod world;

use crate::utils::user_events::UserEvent;
use crate::utils::config::Config;
use crate::utils::fps_counter::FpsCounter;
use crate::utils::math::{self, Isometry3, Vec3};
use crate::application::gui::Gui;
use render::{Render, RenderState};
use bench::Benchmark;
use gpu_bench::GpuBenchmark;
use input::Input;
use world::World;

pub struct Application {
	graphics_state: RenderState,
	pub input: Input,
	pub world: Option<World>,
	pub gui: Option<Gui>,
	pub config: Config,
	pub pov: Isometry3,
	pub pov_rot: (f32, f32, f32),
	pub cpu_bench: Benchmark,
	pub gpu_bench: Option<GpuBenchmark>,
	pub last_frame: Instant,
	fps_counter: FpsCounter,
}

impl Application {
	pub fn new(event_loop: &EventLoop<UserEvent>, config: Config) -> Self {
		let pov_rot = (0.0, 0.0, 0.0);
		let pov = Isometry3::from_parts(Vec3::new(0.0, 64.0, 0.0).into(), math::from_euler(pov_rot.0, pov_rot.1, pov_rot.2));
		
		Self {
			graphics_state: RenderState::new(event_loop),
			input: Input::new(),
			world: None,
			gui: None,
			config,
			pov,
			pov_rot,
			cpu_bench: Benchmark::new(),
			gpu_bench: None,
			last_frame: Instant::now(),
			fps_counter: FpsCounter::new(),
		}
	}
	
	pub fn graphics(&self) -> Option<&Render> {
		match &self.graphics_state {
			RenderState::Ready(graphics) => Some(graphics),
			_ => None,
		}
	}
	
	pub fn graphics_mut(&mut self) -> Option<&mut Render> {
		match &mut self.graphics_state {
			RenderState::Ready(graphics) => Some(graphics),
			_ => None,
		}
	}
	
	fn draw(&mut self) {
		if let Some(graphics) = self.graphics_mut() {
			// graphics.draw();
		}
	}
	
	fn resized(&mut self, size: PhysicalSize<u32>) {
		if let Some(graphics) = self.graphics_mut() {
			graphics.resize(size);
		}
	}
}

impl ApplicationHandler<UserEvent> for Application {
	fn resumed(&mut self, event_loop: &ActiveEventLoop) {
		if self.graphics_state.can_init() {
			self.graphics_state.init(event_loop).expect("Couldn't initialize graphics.");
		}
	}
	
	fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: UserEvent) {
		match event {
			UserEvent::GraphicsReady(render) => {
				render.request_redraw();
				
				self.gui = Some(Gui::new(&render));
				self.world = Some(World::new(&self.config.world_model, &render).expect("Failed to create world"));
				self.gpu_bench = cfg!(not(target_arch = "wasm32")).then(|| GpuBenchmark::new(&render));
				self.graphics_state = RenderState::Ready(render);
			}
		}
	}
	
	fn window_event(
		&mut self,
		event_loop: &ActiveEventLoop,
		_window_id: WindowId,
		event: WindowEvent,
	) {
		match event {
			WindowEvent::Resized(size) => self.resized(size),
			WindowEvent::RedrawRequested => self.draw(),
			WindowEvent::CloseRequested => event_loop.exit(),
			_ => {}
		}
	}
}
