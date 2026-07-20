use std::future::Future;
use anyhow::{anyhow, Result};
use wgpu::web_sys;
use wasm_bindgen::JsCast;
use winit::platform::web::{EventLoopExtWebSys, WindowAttributesExtWebSys};
use winit::application::ApplicationHandler;
use winit::event_loop::EventLoop;
use winit::window::{Window, WindowAttributes};

pub type RcWindow = std::rc::Rc<Window>;

pub fn init_logging() -> Result<()> {
	std::panic::set_hook(Box::new(console_error_panic_hook::hook));
	console_log::init_with_level(log::Level::Error)?;
	
	Ok(())
}

pub fn run_app<T>(app: impl ApplicationHandler<T> + 'static, event_loop: EventLoop<T>) -> Result<()> {
	wasm_bindgen_futures::spawn_local(async move {
		event_loop.spawn_app(app);
	});
	
	Ok(())
}

pub fn set_wgpu_features(required_features: wgpu::Features) -> wgpu::Features {
	required_features
}

pub fn set_window_attributes(win_attr: WindowAttributes) -> Result<WindowAttributes> {
	let canvas = web_sys::window()
		.ok_or_else(|| anyhow!("Can't get js window."))?
		.document()
		.ok_or_else(|| anyhow!("Can't get js document."))?
		.get_element_by_id("canvas")
		.ok_or_else(|| anyhow!("Can't get canvas element."))?
		.dyn_into::<web_sys::HtmlCanvasElement>()
		.map_err(|el| anyhow!("Element of id `canvas` is not HtmlCanvasElement. Got: {}", el.tag_name()))?;
	
	Ok(win_attr.with_canvas(Some(canvas)))
}

pub fn spawn_future(future: impl Future<Output = ()> + 'static) {
	wasm_bindgen_futures::spawn_local(future);
}
