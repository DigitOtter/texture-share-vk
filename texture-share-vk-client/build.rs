use cbindgen::Language;
use std::path::Path;

fn main() {
	// Generate C bindings
	let c_header_filename =
		Path::new(option_env!("TSV_RUST_GEN_INCLUDE_DIR").unwrap_or("../target/gen_include"))
			.join("texture_share_vk/texture_share_vk_client.h");
	cbindgen::Builder::new()
		.with_language(Language::Cxx)
		.with_crate(".")
		.with_pragma_once(true)
		.with_tab_width(4)
		.with_include("texture_share_ipc/texture_share_ipc.h")
		.with_include("texture_share_vk_base.h")
		.generate()
		.expect("Failed to generate bindings")
		.write_to_file(c_header_filename);

	// Generate cxx bindings
	//cxx_build::bridge("src/bindings/bindings_cpp.rs").compile("cxx_vk_client");
}
