use egui::{Context, ViewportId};
use egui_wgpu::{Renderer as EguiRenderer, RendererOptions, ScreenDescriptor};
use egui_winit::State;
use wgpu::{CommandEncoder, TextureView};
use winit::event::WindowEvent;
use winit::window::Window;

use crate::application::render::Render;

pub struct Gui {
	state: State,
	renderer: EguiRenderer,
}

impl Gui {
	pub fn new(render: &Render) -> Self {
		let egui_context = Context::default();
		
		let state = State::new(egui_context, ViewportId::ROOT, &render.window, Some(render.window.scale_factor() as f32), None, None);
		let renderer = EguiRenderer::new(&render.device, render.surface_config.format, RendererOptions::default());
		
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
		self.ctx().begin_pass(raw_input);
	}
	
	pub fn end_frame(&mut self, window: &Window, render: &Render, encoder: &mut CommandEncoder, view: &TextureView) {
		let screen_descriptor = ScreenDescriptor {
			pixels_per_point: self.ctx().pixels_per_point(),
			size_in_pixels: [render.surface_config.width, render.surface_config.height]
		};
		
		let full_output = self.ctx().end_pass();
		self.state.handle_platform_output(&window, full_output.platform_output);
		
		let tris = self.state.egui_ctx().tessellate(full_output.shapes, self.state.egui_ctx().pixels_per_point());
		
		for (id, image_delta) in &full_output.textures_delta.set {
			self.renderer.update_texture(&render.device, &render.queue, *id, &image_delta);
		}
		
		self.renderer.update_buffers(&render.device, &render.queue, encoder, &tris, &screen_descriptor);
		
		let rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
			color_attachments: &[Some(wgpu::RenderPassColorAttachment {
				view: &view,
				resolve_target: None,
				depth_slice: None,
				ops: wgpu::Operations {
					load: wgpu::LoadOp::Load,
					store: wgpu::StoreOp::Store,
				},
			})],
			depth_stencil_attachment: None,
			timestamp_writes: None,
			label: Some("Gui Render Pass"),
			occlusion_query_set: None,
			multiview_mask: None,
		});
		
		self.renderer.render(&mut rpass.forget_lifetime(), &tris, &screen_descriptor);
		
		for x in &full_output.textures_delta.free {
			self.renderer.free_texture(x)
		}
	}
}

impl std::fmt::Debug for Gui {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Gui").finish_non_exhaustive()
	}
}
