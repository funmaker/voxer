use std::sync::Arc;
use std::time::Instant;
use anyhow::Result;
use nalgebra::{Matrix, Translation3, vector};
use winit::dpi::PhysicalSize;
use winit::window::Window;
use input::{Input, Key};
use render_context::RenderContext;
use world::World;

pub mod input;
pub mod render_context;
pub mod world;
pub mod gui;
pub mod shaders;
mod bench;
mod gpu_bench;

use crate::application::gpu_bench::WriteTimestamp;
use crate::utils::config::Config;
use crate::utils::math::{from_euler, Isometry3, PI, Vec3};
use crate::utils::fps_counter::FpsCounter;
use gui::Gui;
use bench::Benchmark;
use gpu_bench::GpuBenchmark;

pub struct Application {
	pub window: Arc<Window>,
	pub gui: Gui,
	pub input: Input,
	pub render: RenderContext,
	pub world: World,
	pub config: Config,
	pub pov: Isometry3,
	pub pov_rot: (f32, f32, f32),
	pub cpu_bench: Benchmark,
	pub gpu_bench: Option<GpuBenchmark>,
	pub last_frame: Instant,
	fps_counter: FpsCounter,
}

impl Application {
	pub async fn new(window: Window, config: Config) -> Result<Self> {
		let window = Arc::new(window);
		let render = RenderContext::new(window.clone()).await?;
		let world = World::new(&config.world_model, &render)?;
		
		let pov_rot = (0.0, 0.0, 0.0);
		let pov = Isometry3::from_parts(Vec3::new(0.0, 64.0, 0.0).into(), from_euler(pov_rot.0, pov_rot.1, pov_rot.2));
		
		let gui = Gui::new(&render, &window);
		let cpu_bench = Benchmark::new();
		let gpu_bench = cfg!(not(target_arch = "wasm32")).then(|| GpuBenchmark::new(&render));
		
		Ok(Application {
			window,
			gui,
			input: Input::new(),
			render,
			world,
			config,
			pov,
			pov_rot,
			cpu_bench,
			gpu_bench,
			last_frame: Instant::now(),
			fps_counter: FpsCounter::new(),
		})
	}
	
	pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
		self.render.resize(new_size);
		self.window.request_redraw();
	}
	
	pub fn tick(&mut self) -> Result<()> {
		let delta_time = self.last_frame.elapsed().as_secs_f32();
		self.last_frame = Instant::now();
		self.cpu_bench.new_frame();
		
		let mut speed = 32.0 * delta_time;
		if self.input.keyboard.pressed(Key::ShiftLeft) { speed *= 2.0; }
		let b2f = |key: Key| if self.input.keyboard.pressed(key) { speed } else { 0.0 };
		
		self.pov *= Translation3::new(
			b2f(Key::KeyD) - b2f(Key::KeyA),
			b2f(Key::Space) - b2f(Key::ControlLeft),
			b2f(Key::KeyS) - b2f(Key::KeyW),
		);
		
		self.pov_rot.0 = (self.pov_rot.0 + self.input.mouse.axis(1) * -0.01).clamp(-PI / 2.0, PI / 2.0);
		self.pov_rot.1 = self.pov_rot.1 + self.input.mouse.axis(0) * -0.01;
		
		self.pov.rotation = from_euler(self.pov_rot.0, self.pov_rot.1, self.pov_rot.2);
		
		if self.input.keyboard.down(Key::KeyP) { self.cpu_bench.toggle_open(); }
		if self.input.keyboard.down(Key::KeyO) {
			if let Some(gpu_bench) = &mut self.gpu_bench {
				gpu_bench.toggle_open();
			}
		}
		
		self.cpu_bench.tick("Logic");
		
		self.render()?;
		
		self.input.reset();
		
		self.cpu_bench.tick("Cleanup");
		
		Ok(())
	}
	
	fn render(&mut self) -> Result<()> {
		self.fps_counter.tick();
		
		if let Some(gpu_bench) = &mut self.gpu_bench {
			gpu_bench.new_frame(&self.render);
		}
		
		let frame = self.render.surface.get_current_texture()?;
		let aspect_ratio = frame.texture.width() as f32 / frame.texture.height() as f32;
		
		self.render.commons.view = self.pov.to_homogeneous() * Matrix::new_nonuniform_scaling(&vector!(1.0, 1.0 / aspect_ratio, 1.0));
		self.render.commons.frame = self.render.commons.frame.wrapping_add(1);
		self.render.update_commons();
		
		let mut encoder = self.render.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("Main Render Encoder") });
		
		let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
		let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
			label: Some("Main Render Pass"),
			color_attachments: &[Some(wgpu::RenderPassColorAttachment {
				view: &view,
				resolve_target: None,
				ops: wgpu::Operations {
					load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
					store: wgpu::StoreOp::Store,
				},
			})],
			depth_stencil_attachment: None,
			timestamp_writes: None,
			occlusion_query_set: None,
		});
		
		self.gpu_bench_tick("Render Setup", &mut rpass);
		self.cpu_bench.tick("Render Setup");
		
		self.world.render(&mut rpass);
		
		// TODO: fix lifetimes
		if let Some(gpu_bench) = &mut self.gpu_bench {
			gpu_bench.tick("Render World", &mut rpass);
		}
		self.cpu_bench.tick("Render World");
		
		drop(rpass);
		
		self.gpu_bench_tick("Pass End", &mut encoder);
		
		self.gui.begin_frame(&self.window, 1.0);
		self.on_gui();
		self.gui.end_frame(&self.window, &self.render, &mut encoder, &view);
		
		self.gpu_bench_tick("Render Gui", &mut encoder);
		self.cpu_bench.tick("Render Gui");
		
		self.render.queue.submit(Some(encoder.finish()));
		frame.present();
		
		self.cpu_bench.tick("Render End");
		
		Ok(())
	}
	
	fn on_gui(&mut self) {
		use egui::*;
		
		let ctx = self.gui.ctx();
		let target_fps = self.window.current_monitor().and_then(|mon| mon.refresh_rate_millihertz().map(|mhz| mhz as f32 * 0.001));
		
		Window::new("Info")
			.title_bar(false)
			.resizable(false)
			.default_pos([4.0, 4.0])
			.frame(Frame {
				inner_margin: Margin::same(8.0),
				outer_margin: Margin::ZERO,
				rounding: Rounding::same(4.0),
				fill: Color32::from_rgba_unmultiplied(0, 0, 0, 200),
				..Frame::default()
			})
			.show(ctx, |ui| {
				ui.label(format!("FPS: {}", self.fps_counter.fps().ceil()));
			});
		
		self.cpu_bench.on_gui_window(ctx, "CPU Timings", target_fps);
		if let Some(gpu_bench) = &mut self.gpu_bench {
			gpu_bench.on_gui_window(ctx, "GPU Timings", target_fps);
		}
	}
	
	fn gpu_bench_tick(&mut self, stage: &'static str, encoder: &mut impl WriteTimestamp) {
		if let Some(gpu_bench) = self.gpu_bench.as_mut() {
			gpu_bench.tick(stage, encoder);
		}
	}
}
