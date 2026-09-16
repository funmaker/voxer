#![feature(layout_for_ptr)]
#![feature(ptr_metadata)]
#![feature(int_roundings)]
#![feature(offset_of_slice)]
#![feature(trivial_bounds)]

use winit::event_loop::{ControlFlow, EventLoop};
use anyhow::Result;

pub mod platform;
pub mod application;
pub mod utils;
pub mod harness;
pub mod shaders;

use crate::harness::ApplicationHarness;
use crate::utils::config::Config;
use crate::utils::user_events::UserEvent;

fn main() -> Result<()> {
    platform::init_logging().expect("Couldn't initialize logger");
    
    let config = Config::from_args();
    
    let event_loop = EventLoop::<UserEvent>::with_user_event().build()?;
    event_loop.set_control_flow(ControlFlow::Poll); // Run continuously
    
    let harness = ApplicationHarness::new(&event_loop, config);
    
    platform::run_app(harness, event_loop)?;
    
    Ok(())
}
