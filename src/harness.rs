use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::{DeviceEvent, DeviceId, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy};
use winit::window::{Window, WindowId};

use crate::platform;
use crate::application::Application;
use crate::utils::config::Config;
use crate::utils::user_events::UserEvent;

pub enum ApplicationHarness {
	Created {
		proxy: EventLoopProxy<UserEvent>,
		config: Config,
	},
	Initializing,
	Ready {
		application: Application,
	},
}

impl ApplicationHarness {
	pub fn new(event_loop: &EventLoop<UserEvent>, config: Config) -> Self {
		ApplicationHarness::Created {
			proxy: event_loop.create_proxy(),
			config
		}
	}
}

impl ApplicationHandler<UserEvent> for ApplicationHarness {
	fn resumed(&mut self, event_loop: &ActiveEventLoop) {
		if let ApplicationHarness::Created { .. } = self {
			let ApplicationHarness::Created {
				proxy,
				config,
			} = std::mem::replace(self, ApplicationHarness::Initializing) else { unreachable!() };
			
			let win_attr =
				Window::default_attributes()
					.with_title("Voxer")
					.with_inner_size(PhysicalSize::new(1280, 720))
					.with_transparent(true);
			
			let win_attr = platform::set_window_attributes(win_attr).expect("Error while setting window attributes.");
			let window = event_loop.create_window(win_attr).expect("Error during window creation.");
			
			platform::spawn_future(async move {
				let application = Application::new(window, config).await.expect("Error during initialization.");
				proxy.send_event(UserEvent::Initialized(application)).expect("Error while sending init event to event loop.");
			});
		}
	}
	
	fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: UserEvent) {
		match event {
			UserEvent::Initialized(application) => {
				*self = ApplicationHarness::Ready { application };
			}
		}
	}
	
	fn window_event(&mut self, event_loop: &ActiveEventLoop, window_id: WindowId, event: WindowEvent) {
		let ApplicationHarness::Ready { application } = self else { return };
		
		application.on_window_event(event_loop, window_id, event).expect("Error in window event handler.");
	}
	
	fn device_event(&mut self, event_loop: &ActiveEventLoop, device_id: DeviceId, event: DeviceEvent) {
		let ApplicationHarness::Ready { application } = self else { return };
		
		application.on_device_event(event_loop, device_id, event).expect("Error in device event handler.");
	}
	
	fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
		let ApplicationHarness::Ready { application } = self else { return };
		
		application.render.window.request_redraw();
	}
}
