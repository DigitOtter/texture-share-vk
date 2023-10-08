use cc::{self, Build};
use cmake;

fn add_cxx_file(mut cfg: Build, cpp_file: &str, h_file: &str) -> Build {
    println!("cargo:rerun-if-changed={}", cpp_file);
    println!("cargo:rerun-if-changed={}", h_file);

    cfg.file(cpp_file).to_owned()
}

fn main() {
    // Build gl library
    let lib_name = "GlSharedImage";
    let mut dst = cmake::Config::new("cpp")
        .always_configure(true)
        .configure_arg("-DBUILD_SHARED_LIBS=False")
        //.init_cxx_cfg(cxx_conf)
        .build();

    println!("cargo:rustc-link-search=native={}", dst.display());
    dst.push("build");
    println!("cargo:rustc-link-search=native={}", dst.display());
    println!("cargo:rustc-link-lib=static={}", lib_name);

    // Link to OpenGl
    //#[cfg(test)]
    //println!("cargo:rustc-link-lib=X11");
    println!("cargo:rustc-link-lib=GL");
    //println!("cargo:rustc-link-lib=GLU");
    //println!("cargo:rustc-link-lib=glut");

    // Generate gl library bindings
    let cxx_rs_files = vec!["src/opengl/gl_shared_image.rs"];

    let cxx_conf = cxx_build::bridges(cxx_rs_files.clone())
        .include("cpp")
        .to_owned();

    let cxx_conf = add_cxx_file(
        cxx_conf.to_owned(),
        "cpp/wrapper/gl_shared_image_wrapper.cpp",
        "cpp/wrapper/gl_shared_image_wrapper.h",
    );

    cxx_conf.compile("rust_gl_shared_image");

    for file in cxx_rs_files.iter() {
        println!("cargo:rerun-if-changed={}", file);
    }

    // Generate base bindings
    //let structs_header_name = "texture_share_vk_base_structs.h";
    // let mut config = cbindgen::Config::default();
    // config.export.exclude = vec![
    //     "VkInstance".to_string(),
    //     "VkPhysicalDevice".to_string(),
    //     "VkDevice".to_string(),
    //     "VkQueue".to_string(),
    //     "VkCommandPool".to_string(),
    //     "VkCommandBuffer".to_string(),
    //     "VkFormat".to_string(),
    // ];
    // cbindgen::Builder::new()
    //     .with_config(config)
    //     .with_crate(".")
    //     .include_item("ShmemInternalData")
    //     .with_pragma_once(true)
    //     .with_tab_width(4)
    //     .with_sys_include("vulkan.h")
    //     .with_include("texture_share_ipc/texture_share_ipc.h")
    //     .with_include(structs_header_name)
    //     .generate()
    //     .expect("Failed to generate bindings")
    //     .write_to_file("../target/gen_include/texture_share_vk/texture_share_vk_base.h");

    // fs::copy(
    //     "cpp/bindings/texture_share_vk_base_structs.h",
    //     format!(
    //         "../target/gen_include/texture_share_vk/{}",
    //         structs_header_name
    //     ),
    // )
    // .expect("Failed to copy files to gen_includes");
}
