use std::fmt;
use std::time::Instant;
use anyhow::{anyhow, Result};
use egui::Ui;
use wgpu::{CurrentSurfaceTexture, SubmissionIndex};
use winit::dpi::PhysicalPosition;
use winit::event::{DeviceEvent, DeviceId, ElementState, KeyEvent, MouseButton, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{CursorGrabMode, Window, WindowId};

pub mod render;
mod bench;
mod gpu_bench;
mod gui;
mod input;
mod world;

use crate::utils::config::Config;
use crate::utils::fps_counter::FpsCounter;
use crate::utils::math::{self, Translation3};
use crate::application::gui::Gui;
use crate::application::gpu_bench::WriteTimestamp;
use crate::application::input::Key;
use render::Render;
use bench::Benchmark;
use gpu_bench::GpuBenchmark;
use input::Input;
use world::World;

pub struct Application {
	pub render: Render,
	pub input: Input,
	pub world: World,
	pub config: Config,
	pub cpu_bench: Benchmark,
	pub gpu_bench: Option<GpuBenchmark>,
	pub last_frame: Instant,
	pub delta_time: f32,
	gui: Option<Gui>,
	fps_counter: FpsCounter,
	cursor_trap: bool,
	last_submission: Option<SubmissionIndex>,
}

impl Application {
	pub async fn new(window: Window, config: Config) -> Result<Self> {
		let render = Render::new(window).await?;
		let world = World::new(&config.world_model, &render)?;
		let gui = Gui::new(&render);
		
		let cpu_bench = Benchmark::new();
		let gpu_bench = cfg!(not(target_arch = "wasm32")).then(|| GpuBenchmark::new(&render));
		
		Ok(Self {
			render,
			input: Input::new(),
			world,
			config,
			cpu_bench,
			gpu_bench,
			last_frame: Instant::now(),
			delta_time: 0.0,
			gui: Some(gui),
			fps_counter: FpsCounter::new(),
			cursor_trap: false,
			last_submission: None,
		})
	}
	
	pub fn on_window_event(&mut self, event_loop: &ActiveEventLoop, _window_id: WindowId, mut event: WindowEvent) -> Result<()> {
		if !self.cursor_trap
		&& let Some(gui) = &mut self.gui
		&& gui.on_event(&self.render.window, &event) {
			return Ok(());
		}
		
		match event {
			WindowEvent::CloseRequested => event_loop.exit(),
			WindowEvent::Resized(size) => self.render.resize(size),
			WindowEvent::RedrawRequested => self.on_tick()?,
			
			WindowEvent::MouseInput {
				button: MouseButton::Left,
				state: ElementState::Pressed, ..
			} if !self.cursor_trap => {
				self.cursor_trap = true;
				self.render.window.set_cursor_visible(false);
				self.render.window.set_cursor_grab(CursorGrabMode::Confined)
				           .or_else(|_| self.render.window.set_cursor_grab(CursorGrabMode::Locked))?;
				
				// When cursor becomes grabbed, egui stops receiving events. Let's fake button release so it doesn't think it's constantly pressed.
				if let WindowEvent::MouseInput { state, .. } = &mut event {
					*state = ElementState::Released;
				}
				
				if let Some(gui) = &mut self.gui {
					gui.on_event(&self.render.window, &event);
				}
			},
			
			WindowEvent::KeyboardInput {
				event: KeyEvent {
					physical_key: PhysicalKey::Code(KeyCode::Escape),
					state: ElementState::Pressed,
					repeat: false, ..
				}, ..
			} if self.cursor_trap => {
				self.cursor_trap = false;
				self.render.window.set_cursor_visible(true);
				self.render.window.set_cursor_grab(CursorGrabMode::None)?;
				
				let size = self.render.window.inner_size();
				let center = PhysicalPosition::new(size.width / 2, size.height / 2);
				self.render.window.set_cursor_position(center)?;
			},
			
			WindowEvent::KeyboardInput {
				event: KeyEvent {
					physical_key: PhysicalKey::Code(key),
					state,
					repeat: false, ..
				}, ..
			} if self.cursor_trap => {
				self.input.keyboard.update_button(key, state == ElementState::Pressed);
			},
			_ => {}
		}
		
		Ok(())
	}
	
	pub fn on_device_event(&mut self, _event_loop: &ActiveEventLoop, _device_id: DeviceId, event: DeviceEvent) -> Result<()> {
		if !self.cursor_trap { return Ok(()); }
		
		match event {
			DeviceEvent::Motion {
				axis,
				value,
			} => {
				let size = self.render.window.inner_size();
				let center = PhysicalPosition::new(size.width / 2, size.height / 2);
				self.render.window.set_cursor_position(center)?;
				
				self.input.mouse.update_axis(axis as usize, value as f32);
			},
			_ => {},
		}
		
		Ok(())
	}
	
	fn on_tick(&mut self) -> Result<()> {
		self.delta_time = self.last_frame.elapsed().as_secs_f32();
		self.last_frame = Instant::now();
		self.cpu_bench.new_frame();
		
		let mut speed = 32.0 * self.delta_time;
		if self.input.keyboard.pressed(Key::ShiftLeft) { speed *= 2.0; }
		let b2f = |key: Key| if self.input.keyboard.pressed(key) { speed } else { 0.0 };
		
		self.render.pov *= Translation3::new(
			b2f(Key::KeyD) - b2f(Key::KeyA),
			b2f(Key::Space) - b2f(Key::ControlLeft),
			b2f(Key::KeyS) - b2f(Key::KeyW),
		);
		
		self.render.pov_rot.0 = (self.render.pov_rot.0 + self.input.mouse.axis(1) * -0.01).clamp(-math::PI / 2.0, math::PI / 2.0);
		self.render.pov_rot.1 = self.render.pov_rot.1 + self.input.mouse.axis(0) * -0.01;
		
		self.render.pov.rotation = math::from_euler(self.render.pov_rot.0, self.render.pov_rot.1, self.render.pov_rot.2);
		
		if self.input.keyboard.down(Key::KeyP) { self.cpu_bench.toggle_open(); }
		if self.input.keyboard.down(Key::KeyO) {
			if let Some(gpu_bench) = &mut self.gpu_bench {
				gpu_bench.toggle_open();
			}
		}
		
		self.cpu_bench.tick("Logic");
		
		self.on_render()?;
		
		self.input.reset();
		
		self.cpu_bench.tick("Cleanup");
		
		Ok(())
	}
	
	fn on_render(&mut self) -> Result<()> {
		self.fps_counter.tick();
		
		if let Some(submission) = self.last_submission.take() {
			self.render.device.poll(wgpu::PollType::Wait {
				submission_index: Some(submission),
				timeout: None,
			})?;
		}
		
		self.cpu_bench.tick("Wait for last frame");
		
		if let Some(gpu_bench) = &mut self.gpu_bench {
			gpu_bench.new_frame(&self.render);
		}
		
		let frame = match self.render.surface.get_current_texture() {
			CurrentSurfaceTexture::Success(surface) |
			CurrentSurfaceTexture::Suboptimal(surface) => surface,
			err => return Err(anyhow!("Failed to acquire next swap chain texture: {:?}", err)),
		};
		
		self.render.commons.frame = self.render.commons.frame.wrapping_add(1);
		self.render.update_commons();
		
		let mut encoder = self.render.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("Main Render Encoder") });
		
		self.gpu_bench_tick("Render Setup", &mut encoder);
		self.cpu_bench.tick("Render Setup");
		
		self.world.before_render(&mut encoder, &self.render);
		
		self.gpu_bench_tick("Before Render", &mut encoder);
		self.cpu_bench.tick("Before Render");
		
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
				depth_slice: None,
			})],
			depth_stencil_attachment: None,
			timestamp_writes: None,
			occlusion_query_set: None,
			multiview_mask: None,
		});
		
		self.world.render(&mut rpass, &self.render);
		
		// TODO: fix lifetimes
		if let Some(gpu_bench) = &mut self.gpu_bench {
			gpu_bench.tick("Render World", &mut rpass);
		}
		self.cpu_bench.tick("Render World");
		
		drop(rpass);
		
		self.gpu_bench_tick("Pass End", &mut encoder);
		
		self.world.after_render(&mut encoder);
		
		self.gpu_bench_tick("After Render", &mut encoder);
		self.cpu_bench.tick("After Render");
		
		self.with_gui(|app, gui| gui.on_render(app, &mut encoder, &view, |app, ui| app.on_gui(ui)))?;
		
		self.gpu_bench_tick("Render Gui", &mut encoder);
		self.cpu_bench.tick("Render Gui");
		
		self.last_submission = Some(self.render.queue.submit(Some(encoder.finish())));
		self.render.queue.present(frame);
		
		self.cpu_bench.tick("Render End");
		
		Ok(())
	}
	
	fn on_gui(&mut self, ui: &mut Ui) -> Result<()> {
		use egui::*;
		
		let target_fps = self.render.window.current_monitor().and_then(|mon| mon.refresh_rate_millihertz().map(|mhz| mhz as f32 * 0.001));
		
		Window::new("Info")
			.title_bar(false)
			.resizable(false)
			.default_pos([4.0, 4.0])
			.frame(Frame {
				inner_margin: Margin::same(8),
				outer_margin: Margin::ZERO,
				corner_radius: CornerRadius::same(4),
				fill: Color32::from_rgba_unmultiplied(0, 0, 0, 200),
				..Frame::default()
			})
			.show(ui, |ui| {
				ui.label(format!("FPS: {}", self.fps_counter.fps().ceil()));
				
				{
					let mut old_val = math::from_radians(self.render.fov);
					if ui.add(Slider::new(&mut old_val, 0.1..=179.0).text("FoV")).changed() {
						self.render.fov = math::to_radians(old_val);
					}
				}
				
				ui.separator();
				
				self.world.gui(ui);
			});
		
		self.cpu_bench.on_gui_window(ui, "CPU Timings", target_fps);
		if let Some(gpu_bench) = &mut self.gpu_bench {
			gpu_bench.on_gui_window(ui, "GPU Timings", target_fps);
		}
		
		Ok(())
	}
	
	fn gpu_bench_tick(&mut self, stage: &'static str, encoder: &mut impl WriteTimestamp) {
		if let Some(gpu_bench) = self.gpu_bench.as_mut() {
			gpu_bench.tick(stage, encoder);
		}
	}
	
	fn with_gui<R>(&mut self, callback: impl FnOnce(&mut Self, &mut Gui) -> R) -> R {
		let mut gui = self.gui.take().expect("nested with_gui call");
		
		let ret = callback(self, &mut gui);
		
		self.gui.replace(gui);
		
		ret
	}
}

impl fmt::Debug for Application {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("Application").finish()
	}
}
