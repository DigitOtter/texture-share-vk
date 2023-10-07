fn main() {
    cbindgen::Builder::new()
        .with_crate(".")
        .with_pragma_once(true)
        .with_tab_width(4)
        //.with_header("texture_share_ipc.h")
        .generate()
        .expect("Failed to generate bindings")
        .write_to_file("../target/gen_include/texture_share_vk/texture_share_vk_client.h");
}
