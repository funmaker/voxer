#![feature(never_type)]
#![feature(set_ptr_value)]
#![feature(ptr_metadata)]
#![feature(try_blocks)]
#![feature(int_roundings)]
#![feature(layout_for_ptr)]

use anyhow::Result;
use winit::dpi::PhysicalPosition;
use winit::dpi::PhysicalSize;
use winit::event::{DeviceEvent, ElementState, Event, KeyEvent, MouseButton, WindowEvent};
use winit::event_loop::EventLoop;
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{CursorGrabMode, Window};

mod utils;
mod application;

use application::Application;
use utils::config::Config;

async fn run(event_loop: EventLoop<()>, window: Window, config: Config) -> Result<()> {
	let mut application = Application::new(window, config).await?;
	let mut cursor_trap = false;
	
	event_loop.run(move |event, target| {
		
		if let Event::WindowEvent { event, .. } = &event {
			if !cursor_trap {
				if application.gui.on_event(&application.window, &event) {
					return;
				}
			}
		}
		
		let result: Result<_> = try {
			match event {
				Event::WindowEvent { event: WindowEvent::Resized(new_size), .. } => {
					application.resize(new_size);
				},
				
				Event::WindowEvent { event: WindowEvent::RedrawRequested, .. } => {
					application.tick()?;
				},
				
				Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
					target.exit();
				},
				
				Event::WindowEvent { event: mut window_event @ WindowEvent::MouseInput {
					button: MouseButton::Left,
					state: ElementState::Pressed, ..
				}, .. } if !cursor_trap => {
					cursor_trap = true;
					application.window.set_cursor_visible(false);
					application.window.set_cursor_grab(CursorGrabMode::Confined)?;
					
					// When cursor is grabbed, egui stops receiving events. Let's fake button release so it doesn't think it's constantly pressed.
					if let WindowEvent::MouseInput { state, .. } = &mut window_event {
						*state = ElementState::Released;
					}
					application.gui.on_event(&application.window, &window_event);
				},
				
				Event::WindowEvent { event: WindowEvent::KeyboardInput {
					event: KeyEvent {
						physical_key: PhysicalKey::Code(KeyCode::Escape),
						state: ElementState::Pressed,
						repeat: false, ..
					}, ..
				}, .. } if cursor_trap => {
					cursor_trap = false;
					application.window.set_cursor_visible(true);
					application.window.set_cursor_grab(CursorGrabMode::None)?;
					
					let size = application.window.inner_size();
					let center = PhysicalPosition::new(size.width / 2, size.height / 2);
					application.window.set_cursor_position(center)?;
				},
				
				Event::WindowEvent { event: WindowEvent::KeyboardInput {
					event: KeyEvent {
						physical_key: PhysicalKey::Code(key),
						state,
						repeat: false, ..
					}, ..
				}, .. } if cursor_trap => {
					application.input.keyboard.update_button(key, state == ElementState::Pressed);
				},
				
				Event::DeviceEvent {
					event: DeviceEvent::Motion {
						axis,
						value,
					}, ..
				} if cursor_trap => {
					let size = application.window.inner_size();
					let center = PhysicalPosition::new(size.width / 2, size.height / 2);
					application.window.set_cursor_position(center)?;
					
					application.input.mouse.update_axis(axis as usize, value as f32);
				}
				
				Event::AboutToWait => {
					application.window.request_redraw();
				}
				
				_ => {}
			}
		};
		
		if let Err(err) = result {
			log::error!("Runtime error: {}", err);
			target.exit();
		}
	}).unwrap();
	
	Ok(())
}

pub fn main() -> Result<()> {
	#[cfg(not(target_arch = "wasm32"))] {
		env_logger::init();
	}
	#[cfg(target_arch = "wasm32")] {
		log::set_max_level(log::LevelFilter::Trace);
		std::panic::set_hook(Box::new(console_error_panic_hook::hook));
		console_log::init().expect("could not initialize logger");
	}
	
	let event_loop = EventLoop::new()?;
	let builder = winit::window::WindowBuilder::new();
	
	#[cfg(not(target_arch = "wasm32"))]
	let builder = builder.with_inner_size(PhysicalSize::new(1280, 720))
	                     .with_transparent(true);
	#[cfg(target_arch = "wasm32")]
	let builder = {
		use wasm_bindgen::JsCast;
		use wgpu::web_sys;
		use winit::platform::web::WindowBuilderExtWebSys;
		let canvas = web_sys::window()
			.unwrap()
			.document()
			.unwrap()
			.get_element_by_id("canvas")
			.unwrap()
			.dyn_into::<web_sys::HtmlCanvasElement>()
			.unwrap();
		builder.with_canvas(Some(canvas))
	};
	
	let window = builder.build(&event_loop)?;
	
	#[cfg(not(target_arch = "wasm32"))] {
		pollster::block_on(run(event_loop, window, Config::from_args()))?;
	}
	#[cfg(target_arch = "wasm32")] {
		wasm_bindgen_futures::spawn_local(async move {
			if let Err(err) = run(event_loop, window, Config::default()).await {
				log::error!("Runtime error: {}", err);
			}
		});
	}
	
	Ok(())
}