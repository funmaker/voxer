pub struct Config {
	pub world_model: String,
}

impl Config {
	#[cfg(not(target_arch = "wasm32"))]
	pub fn from_args() -> Self {
		let defaults = Config::default();
		let args: Vec<_> = std::env::args().collect();
		
		Config {
			world_model: args.get(1).cloned().unwrap_or(defaults.world_model),
		}
	}
}

impl Default for Config {
	fn default() -> Self {
		Self {
			world_model: "vox/colors.vox".to_string(),
		}
	}
}
