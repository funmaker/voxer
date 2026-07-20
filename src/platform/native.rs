use std::future::Future;
use anyhow::Result;
use winit::application::ApplicationHandler;
use winit::event_loop::EventLoop;
use winit::window::{Window, WindowAttributes};

pub type RcWindow = std::sync::Arc<Window>;

pub fn init_logging() -> Result<()> {
	env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("error")).init();
	
	Ok(())
}

pub fn run_app<T>(mut app: impl ApplicationHandler<T> + 'static, event_loop: EventLoop<T>) -> anyhow::Result<()> {
	event_loop.run_app(&mut app)?;
	
	Ok(())
}

pub fn set_window_attributes(win_attr: WindowAttributes) -> anyhow::Result<WindowAttributes> {
	let win_attr = win_attr.with_title("WebGPU example");
	
	Ok(win_attr)
}

pub fn set_wgpu_features(required_features: wgpu::Features) -> wgpu::Features {
	required_features
		| wgpu::Features::TIMESTAMP_QUERY
		| wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS
		| wgpu::Features::TIMESTAMP_QUERY_INSIDE_PASSES
}

pub fn spawn_future(future: impl Future<Output = ()> + 'static) {
	pollster::block_on(future);
}
