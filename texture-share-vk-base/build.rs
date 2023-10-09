use std::{fs, path::Path};

use cbindgen::Language;
use cc::{self, Build};
use cmake;

fn add_cxx_file(mut cfg: Build, cpp_file: &str, h_file: &str) -> Build {
    println!("cargo:rerun-if-changed={}", cpp_file);
    println!("cargo:rerun-if-changed={}", h_file);

    cfg.file(cpp_file).to_owned()
}

fn main() {
    // Build vulkan library
    let lib_name = "VkSharedImage";
    let mut dst = cmake::Config::new("cpp")
        .always_configure(true)
        .configure_arg("-DBUILD_SHARED_LIBS=False")
        //.init_cxx_cfg(cxx_conf)
        .build();

    dst.push("build");

    println!("cargo:warning={}", dst.display());
    println!("cargo:rustc-link-search=native={}", dst.display());
    println!("cargo:rustc-link-lib=static={}", lib_name);

    // Link to third-party library
    dst.push("third_party/vk-bootstrap");
    println!("cargo:rustc-link-search=native={}", dst.display());
    println!("cargo:rustc-link-lib=static={}", "vk-bootstrap");

    println!("cargo:rustc-link-lib={}", "vulkan");

    // Generate vulkan library bindings
    let cxx_rs_files = vec!["src/vulkan/vk_shared_image.rs", "src/vulkan/vk_setup.rs"];

    let cxx_conf = cxx_build::bridges(cxx_rs_files.clone())
        .include("cpp")
        .to_owned();

    let cxx_conf = add_cxx_file(
        cxx_conf.to_owned(),
        "cpp/wrapper/vk_shared_image_wrapper.cpp",
        "cpp/wrapper/vk_shared_image_wrapper.h",
    );
    let cxx_conf = add_cxx_file(
        cxx_conf.to_owned(),
        "cpp/wrapper/vk_setup_wrapper.cpp",
        "cpp/wrapper/vk_setup_wrapper.h",
    );

    cxx_conf.compile("rust_vk_shared_image");

    for file in cxx_rs_files.iter() {
        println!("cargo:rerun-if-changed={}", file);
    }

    // Generate base bindings
    if let Some(c_header_dir) = option_env!("TSV_RUST_GEN_INCLUDE_DIR") {
        let c_header_dir = Path::new(c_header_dir);
        let structs_header_name = "texture_share_vk/texture_share_vk_base_structs.h";

        let mut config = cbindgen::Config::default();
        config.export.exclude = vec![
            "VkInstance".to_string(),
            "VkPhysicalDevice".to_string(),
            "VkDevice".to_string(),
            "VkQueue".to_string(),
            "VkCommandPool".to_string(),
            "VkCommandBuffer".to_string(),
            "VkFormat".to_string(),
        ];
        cbindgen::Builder::new()
            .with_language(Language::C)
            .with_config(config)
            .with_crate(".")
            .include_item("ShmemInternalData")
            .with_pragma_once(true)
            .with_tab_width(4)
            .with_sys_include("vulkan/vulkan.h")
            .with_include("texture_share_ipc/texture_share_ipc.h")
            .with_include(structs_header_name)
            .generate()
            .expect("Failed to generate bindings")
            .write_to_file(&c_header_dir.join("texture_share_vk/texture_share_vk_base.h"));

        fs::copy(
            "cpp/bindings/texture_share_vk_base_structs.h",
            c_header_dir.join(structs_header_name),
        )
        .expect("Failed to copy files to gen_includes");
    }
}
