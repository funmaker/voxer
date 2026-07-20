use crate::application::render::Render;

#[derive(Debug)]
pub enum UserEvent {
	GraphicsReady(Render),
}
