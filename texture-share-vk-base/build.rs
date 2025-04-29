use std::{fs, path::Path};

use cbindgen::Language;

fn main() {
	println!("cargo:rustc-link-lib={}", "vulkan");

	// Generate base bindings
	if let Some(c_header_dir) = option_env!("TSV_RUST_GEN_INCLUDE_DIR") {
		let c_header_dir = Path::new(c_header_dir);
		let _ = fs::create_dir(c_header_dir);

		let mut config = cbindgen::Config::default();
		config.export.exclude = vec![
			"VkInstance",
			"VkPhysicalDevice",
			"VkDevice",
			"VkQueue",
			"VkCommandPool",
			"VkCommandBuffer",
			"VkFormat",
			"VkFence",
			"VkImage",
			"VkImageLayout",
			"VkOffset3D",
		]
		.into_iter()
		.map(|x| x.to_string())
		.collect();
		cbindgen::Builder::new()
			.with_config(config)
			.with_language(Language::C)
			.with_crate(".")
			.include_item("ShmemInternalData")
			.with_pragma_once(true)
			.with_tab_width(4)
			.with_sys_include("vulkan/vulkan.h")
			.with_include("texture_share_ipc/texture_share_ipc.h")
			//.with_include(structs_header_name)
			.generate()
			.expect("Failed to generate bindings")
			.write_to_file(&c_header_dir.join("texture_share_vk/texture_share_vk_base.h"));

		// fs::copy(
		// 	"cpp/bindings/texture_share_vk_base_structs.h",
		// 	c_header_dir.join(structs_header_name),
		// )
		// .expect("Failed to copy files to gen_includes at {:}");
	}
}
