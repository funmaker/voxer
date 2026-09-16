use std::error::Error;
use std::{env, fs};
use std::path::PathBuf;
use wgsl_kiss::{Module, WriteOptions, NalgebraWgslTypeMap};
use wesl::{Mangler, Wesl};
use wesl::syntax::PathOrigin;

const ENTRY_POINTS: &[&str] = &[
	"graphics::world",
	"compute::diffuse",
];

fn main() -> Result<(), Box<dyn Error>> {
	println!("cargo::rerun-if-changed=src/shaders");
	
	let compiler = Wesl::new("src/shaders");
	let mut root = Module::default();
	
	let options = WriteOptions {
		derive_bytemuck_vertex: true,
		type_map: NalgebraWgslTypeMap,
		..Default::default()
	};
	
	for entry_point in ENTRY_POINTS {
		let path = wgsl_kiss::ModulePath {
			components: entry_point.split("::").map(str::to_owned).collect()
		};
		
		let shader = compiler
			.compile(&wesl::ModulePath::new(PathOrigin::Absolute, path.components.clone()))
			.inspect_err(|err| eprintln!("{}", err))?
			.to_string();
		
		root.add_shader_module(&shader, None, options, path, demangle_wesl)?;
	}
	
	let output = root.to_generated_bindings(options);
	
	let out_dir = PathBuf::from(env::var("OUT_DIR")?);
	fs::write(out_dir.join("shaders.rs"), output)?;
	
	Ok(())
}

fn demangle_wesl(name: &str) -> wgsl_kiss::TypePath {
	// todo handle `super` paths
	// Assume all paths are absolute paths.
	if name.starts_with("package_") {
		// Use the root module if unmangle fails.
		let mangler = wesl::EscapeMangler;
		let (path, name) = mangler
			.unmangle(name)
			.unwrap_or((wesl::ModulePath::new_root(), name.to_string()));
		
		// Assume all wesl paths are absolute paths.
		wgsl_kiss::TypePath {
			parent: wgsl_kiss::ModulePath {
				components: path.components,
			},
			name,
		}
	} else {
		// Use the root module if the name is not mangled.
		wgsl_kiss::TypePath {
			parent: wgsl_kiss::ModulePath::default(),
			name: name.to_string(),
		}
	}
}
