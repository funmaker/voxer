use egui::{Context, ViewportId};
use egui_wgpu::{Renderer, ScreenDescriptor};
use egui_winit::State;
use wgpu::{CommandEncoder, TextureView};
use winit::event::WindowEvent;
use winit::window::Window;

use crate::application::render_context::RenderContext;

pub struct Gui {
	state: State,
	renderer: Renderer,
}

impl Gui {
	pub fn new(render: &RenderContext, window: &Window) -> Self {
		let egui_context = Context::default();
		
		let state = State::new(egui_context, ViewportId::ROOT, &window, Some(window.scale_factor() as f32), None);
		let renderer = Renderer::new(&render.device, render.config.format, None, 1);
		
		Gui {
			state,
			renderer,
		}
	}
	
	pub fn ctx(&self) -> &Context {
		self.state.egui_ctx()
	}
	
	pub fn on_event(&mut self, window: &Window, event: &WindowEvent) -> bool {
		self.state.on_window_event(window, &event).consumed
	}
	
	pub fn begin_frame(&mut self, window: &Window, scale_factor: f32) {
		self.ctx().set_pixels_per_point(window.scale_factor() as f32 * scale_factor);
		
		let raw_input = self.state.take_egui_input(&window);
		self.ctx().begin_frame(raw_input);
	}
	
	pub fn end_frame(&mut self, window: &Window, render: &RenderContext, encoder: &mut CommandEncoder, view: &TextureView) {
		let screen_descriptor = ScreenDescriptor {
			pixels_per_point: self.ctx().pixels_per_point(),
			size_in_pixels: [render.config.width, render.config.height]
		};
		
		let full_output = self.ctx().end_frame();
		self.state.handle_platform_output(&window, full_output.platform_output);
		
		let tris = self.state.egui_ctx().tessellate(full_output.shapes, self.state.egui_ctx().pixels_per_point());
		
		for (id, image_delta) in &full_output.textures_delta.set {
			self.renderer.update_texture(&render.device, &render.queue, *id, &image_delta);
		}
		
		self.renderer.update_buffers(&render.device, &render.queue, encoder, &tris, &screen_descriptor);
		
		let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
			color_attachments: &[Some(wgpu::RenderPassColorAttachment {
				view: &view,
				resolve_target: None,
				ops: wgpu::Operations {
					load: wgpu::LoadOp::Load,
					store: wgpu::StoreOp::Store,
				},
			})],
			depth_stencil_attachment: None,
			timestamp_writes: None,
			label: Some("Gui Render Pass"),
			occlusion_query_set: None,
		});
		
		self.renderer.render(&mut rpass, &tris, &screen_descriptor);
		
		drop(rpass);
		
		for x in &full_output.textures_delta.free {
			self.renderer.free_texture(x)
		}
	}
}
