use std::path::Path;

use cbindgen::{self, Language};

fn main() {
	if let Some(c_header_filename) = option_env!("TSV_RUST_GEN_INCLUDE_DIR") {
		let c_header_filename =
			Path::new(c_header_filename).join("texture_share_ipc/texture_share_ipc.h");

		cbindgen::Builder::new()
			.with_language(Language::C)
			.with_crate(".")
			.include_item("ShmemInternalData")
			.with_pragma_once(true)
			.with_tab_width(4)
			//.with_header("texture_share_ipc.h")
			.generate()
			.expect("Failed to generate bindings")
			.write_to_file(&c_header_filename);
	}
}
