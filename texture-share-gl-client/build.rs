use itertools::Itertools;
use std::{
	env,
	path::{Path, PathBuf},
};

use cbindgen::Language;
use cmake;

fn main() {
	// Build gl binding library
	#[cfg(target_os = "linux")]
	let gl_extensions = [
		"GL_EXT_memory_object",
		"GL_EXT_memory_object_fd",
		"GL_EXT_semaphore",
		"GL_EXT_semaphore_fd",
		"GL_EXT_texture_storage",
	];

	let lib_name = "glad";
	let dst = cmake::Config::new("c")
		.always_configure(true)
		.configure_arg("-DBUILD_SHARED_LIBS=OFF")
		.configure_arg("-DGLAD_API=")
		.configure_arg("-DGLAD_GENERATOR=c")
		.configure_arg(format!(
			"-DGLAD_EXTENSIONS={}",
			gl_extensions.into_iter().format(",")
		))
		.configure_arg("-DGLAD_SPEC=gl")
		.configure_arg("-DGLAD_INSTALL=ON")
		.configure_arg("-DGLAD_PROFILE=core")
		.build();

	println!(
		"cargo:rustc-link-search=native={}",
		dst.join("lib").display()
	);
	println!("cargo:rustc-link-lib=static={}", lib_name);

	// Generate glad bindings
	let header_path = dst.join("include/glad/glad.h");
	println!("cargo:rerun-if-changed={}", header_path.display());
	let bindings = bindgen::Builder::default()
		.header(header_path.to_str().unwrap())
		.parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
		.generate()
		.expect("Unable to generate glad bindings");

	let bindings_out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
	bindings
		.write_to_file(bindings_out_path.join("glad_bindings.rs"))
		.expect("Couldn't write glad bindings");

	// Link to OpenGl
	//#[cfg(test)]
	//println!("cargo:rustc-link-lib=X11");
	println!("cargo:rustc-link-lib=GL");
	//println!("cargo:rustc-link-lib=GLU");
	//println!("cargo:rustc-link-lib=glut");

	// Generate base bindings
	let c_header_filename =
		Path::new(option_env!("TSV_RUST_GEN_INCLUDE_DIR").unwrap_or("../target/gen_include"))
			.join("texture_share_gl/texture_share_gl_client.h");
	let mut cgen_config = cbindgen::Config::default();
	cgen_config.export.exclude = vec![
		"GLuint".to_string(),
		"GLenum".to_string(),
		"GLsizei".to_string(),
	];
	cbindgen::Builder::new()
		.with_config(cgen_config)
		.with_language(Language::Cxx)
		.with_crate(".")
		.with_pragma_once(true)
		.with_tab_width(4)
		.with_sys_include("GL/gl.h")
		.with_include("texture_share_ipc/texture_share_ipc.h")
		.generate()
		.expect("Failed to generate bindings")
		.write_to_file(c_header_filename);
}
