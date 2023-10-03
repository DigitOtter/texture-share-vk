#[cxx::bridge]
mod ffi {
    struct SharedImageData {
        id: u32,
        width: u32,
        height: u32,
        format: u32,
        allocation_size: u32,
    }

    extern "Rust" {}

    unsafe extern "C++" {
        include!("vk_shared_image/vk_shared_image.h");

        type SharedImageData;

        type VkSharedImage;
        fn vk_shared_image_new() -> UniquePtr<VkSharedImage>;

        #[rust_name = "cleanup"]
        fn Cleanup(self: Pin<&mut VkSharedImage>);

        #[rust_name = "get_image_data"]
        fn ImageData(self: &VkSharedImage) -> &SharedImageData;

        #[rust_name = "get_image_data_mut"]
        fn ImageData(self: Pin<&mut VkSharedImage>) -> &mut SharedImageData;
    }
}

#[cfg(test)]
mod tests {
    use super::ffi::vk_shared_image_new;
    use super::ffi::SharedImageData;

    #[test]
    fn vk_shared_image_create() {
        let _ = vk_shared_image_new();
    }

    #[test]
    fn vk_shared_image_cleanup() {
        let mut vk_shared_image = vk_shared_image_new();
        vk_shared_image.as_mut().unwrap().cleanup();
    }

    #[test]
    fn vk_shared_image_data() {
        let mut vk_shared_image = vk_shared_image_new();
        const TEST_VAL: u32 = 12345;

        {
            let mut sh_dat = vk_shared_image.as_mut().unwrap().get_image_data_mut();
            sh_dat.id = TEST_VAL;
        }

        {
            let sh_dat = vk_shared_image.get_image_data();
            assert_eq!(sh_dat.id, TEST_VAL);
        }
    }

    // #[test]
    // fn vk_shared_image_bridge_data() {
    //     let vk_shared_image = vk_shared_image_new();
    //     unsafe { vk_shared_image.as_ref().unwrap().ImageData() };
    // }
}
