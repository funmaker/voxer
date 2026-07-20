#![feature(layout_for_ptr)]
#![feature(ptr_metadata)]
#![feature(int_roundings)]

use winit::event_loop::{ControlFlow, EventLoop};
use anyhow::Result;

pub mod platform;
mod application;
mod utils;

use crate::application::Application;
use crate::utils::config::Config;
use crate::utils::user_events::UserEvent;

fn main() -> Result<()> {
    platform::init_logging().expect("Couldn't initialize logger");
    
    let config = Config::default();
    
    let event_loop = EventLoop::<UserEvent>::with_user_event().build()?;
    event_loop.set_control_flow(ControlFlow::Poll); // Run continuously
    
    platform::run_app(Application::new(&event_loop, config), event_loop)?;
    
    Ok(())
}
