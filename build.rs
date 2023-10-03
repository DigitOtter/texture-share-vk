use cmake;

fn main() {
    let cxx_rs_file = "src/vulkan/vk_shared_image.rs";
    let cxx_cpp_file = "cpp/vk_shared_image/vk_shared_image.cpp";
    let cxx_h_file = "cpp/vk_shared_image/vk_shared_image.h";

    let cxx_conf = cxx_build::bridge(cxx_rs_file)
        .file(cxx_cpp_file)
        .include("cpp")
        //.to_owned();
        .compile("rust_vk_shared_image");

    println!("cargo:rerun-if-changed={}", cxx_rs_file);
    println!("cargo:rerun-if-changed={}", cxx_cpp_file);
    println!("cargo:rerun-if-changed={}", cxx_h_file);

    let lib_name = "VkSharedImage";
    let mut dst = cmake::Config::new("cpp")
        //.always_configure(true)
        //.configure_arg("-DBUILD_SHARED_LIBS=False")
        //.init_cxx_cfg(cxx_conf)
        .build();

    dst.push("build");

    println!("cargo:warning={}", dst.display());
    println!("cargo:rustc-link-search=native={}", dst.display());
    println!("cargo:rustc-link-lib={}", lib_name);

    println!("cargo:rustc-link-lib={}", "vulkan");
}
