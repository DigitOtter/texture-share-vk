use cc::{self, Build};
use cmake;

fn add_cxx_file(mut cfg: Build, cpp_file: &str, h_file: &str) -> Build {
    println!("cargo:rerun-if-changed={}", cpp_file);
    println!("cargo:rerun-if-changed={}", h_file);

    cfg.file(cpp_file).to_owned()
}

fn main() {
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

    dst.push("third_party/vk-bootstrap");
    println!("cargo:rustc-link-search=native={}", dst.display());
    println!("cargo:rustc-link-lib=static={}", "vk-bootstrap");

    println!("cargo:rustc-link-lib={}", "vulkan");

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
}
