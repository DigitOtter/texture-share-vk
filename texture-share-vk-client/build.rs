fn main() {
    // Generate C bindings
    cbindgen::Builder::new()
        .with_crate(".")
        .with_pragma_once(true)
        .with_tab_width(4)
        .with_include("texture_share_ipc/texture_share_ipc.h")
        .with_include("texture_share_vk_base.h")
        .generate()
        .expect("Failed to generate bindings")
        .write_to_file("../target/gen_include/texture_share_vk/texture_share_vk_client.h");

    // Generate cxx bindings
    cxx_build::bridge("src/bindings/bindings_cpp.rs").compile("cxx_vk_client");
}
