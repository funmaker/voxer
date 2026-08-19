use egui::{Context, Ui, ViewportId};
use egui_wgpu::{Renderer as EguiRenderer, RendererOptions, ScreenDescriptor};
use egui_winit::State;
use wgpu::{CommandEncoder, TextureView};
use winit::event::WindowEvent;
use winit::window::Window;
use crate::application::Application;
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
	
	pub fn on_render<E>(
		&mut self,
		app: &mut Application,
		encoder: &mut CommandEncoder,
		view: &TextureView,
		callback: impl Fn(&mut Application, &mut Ui) -> Result<(), E>,
	) -> Result<(), E> {
		self.ctx().set_pixels_per_point(app.render.window.scale_factor() as f32);
		
		let mut error = None;
		let raw_input = self.state.take_egui_input(&app.render.window);
		let mut full_output = self.ctx().run_ui(raw_input, |ui| {
			let result = callback(app, ui);
			if let Err(err) = result {
				error = Some(err);
				ui.ctx().request_discard("Error during on_render callback");
			}
		});
		
		if let Some(err) = error {
			return Err(err);
		}
		
		let screen_descriptor = ScreenDescriptor {
			pixels_per_point: self.ctx().pixels_per_point(),
			size_in_pixels: [app.render.surface_config.width, app.render.surface_config.height]
		};
		
		self.state.handle_platform_output(&app.render.window, full_output.platform_output);
		
		let tris = self.state.egui_ctx().tessellate(full_output.shapes, self.state.egui_ctx().pixels_per_point());
		
		for (id, image_deltas) in full_output.textures_delta.set.drain() {
			for image_delta in image_deltas {
				self.renderer.update_texture(&app.render.device, &app.render.queue, id, &image_delta);
			}
		}
		
		self.renderer.update_buffers(&app.render.device, &app.render.queue, encoder, &tris, &screen_descriptor);
		
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
		
		for x in full_output.textures_delta.free.drain() {
			self.renderer.free_texture(&x)
		}
		
		Ok(())
	}
}

impl std::fmt::Debug for Gui {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Gui").finish_non_exhaustive()
	}
}
