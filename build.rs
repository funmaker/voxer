use std::error::Error;
use std::{env, fs};
use std::path::PathBuf;
use wgsl_kiss::{Module, WriteOptions, ModulePath, NalgebraWgslTypeMap};
use wesl::{Mangler, Wesl};

fn main() -> Result<(), Box<dyn Error>> {
	println!("cargo::rerun-if-changed=src/shaders");
	
	let compiler = Wesl::new("src/shaders");
	
	let world = compiler.compile(&"package::world".parse()?)
		.inspect_err(|err| eprintln!("{}", err))?
		.to_string();
	
	let options = WriteOptions {
		derive_bytemuck_vertex: true,
		type_map: NalgebraWgslTypeMap,
		..Default::default()
	};
	
	let mut root = Module::default();
	
	root.add_shader_module(&world, None, options, module_path("world"), demangle_wesl)?;
	
	let output = root.to_generated_bindings(options);
	
	let out_dir = PathBuf::from(env::var("OUT_DIR")?);
	fs::write(out_dir.join("shaders.rs"), output)?;
	// fs::write("src/shaders/mod.rs", output)?;
	
	Ok(())
}

fn module_path(name: &str) -> ModulePath {
	ModulePath { components: vec![name.to_owned()] }
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
			parent: ModulePath {
				components: path.components,
			},
			name,
		}
	} else {
		// Use the root module if the name is not mangled.
		wgsl_kiss::TypePath {
			parent: ModulePath::default(),
			name: name.to_string(),
		}
	}
}
