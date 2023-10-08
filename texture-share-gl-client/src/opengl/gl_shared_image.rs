use self::ffi::{GlFormat, SharedImageData};
use cxx::{type_id, ExternType};
use std::ffi::{c_int, c_uint};
use texture_share_ipc::platform::ShmemDataInternal;

#[repr(transparent)]
pub struct GLuint(pub c_uint);

unsafe impl ExternType for GLuint {
    type Id = type_id!("GLuint");
    type Kind = cxx::kind::Trivial;
}

#[repr(transparent)]
pub struct GLenum(pub c_uint);

unsafe impl ExternType for GLenum {
    type Id = type_id!("GLenum");
    type Kind = cxx::kind::Trivial;
}

#[repr(transparent)]
pub struct GLsizei(pub c_int);

unsafe impl ExternType for GLsizei {
    type Id = type_id!("GLsizei");
    type Kind = cxx::kind::Trivial;
}

#[repr(transparent)]
pub struct GLuint64(pub u64);

unsafe impl ExternType for GLuint64 {
    type Id = type_id!("GLuint64");
    type Kind = cxx::kind::Trivial;
}

#[cxx::bridge]
pub mod ffi {
    #[repr(u32)]
    #[derive(Debug)]
    enum GlFormat {
        RGBA = 0x1908,
        BGRA = 0x80E1,
        RGB = 0x1907,
        BGR = 0x80E0,
        FALSE = 0x0000,
    }

    struct SharedImageData {
        pub id: u32,
        pub width: u32,
        pub height: u32,
        pub format: GlFormat,
        pub allocation_size: u32,
    }

    extern "Rust" {}

    unsafe extern "C++" {
        include!("wrapper/gl_shared_image_wrapper.h");

        #[rust_name = "ShareHandles"]
        type ShareHandlesWrapper;

        type SharedImageData;
        type GlFormat;
        type GLuint = super::GLuint;
        type GLenum = super::GLenum;
        type GLsizei = super::GLsizei;
        type GLuint64 = super::GLuint64;

        type ImageExtent;

        #[rust_name = "GlSharedImage"]
        type GlSharedImageWrapper;

        fn gl_external_initialize() -> bool;

        fn gl_share_handles_new() -> UniquePtr<ShareHandles>;
        fn gl_share_handles_from_fd(memory_fd: i32) -> UniquePtr<ShareHandles>;

        fn get_memory_handle(self: &ShareHandles) -> i32;
        fn release_memory_handle(self: Pin<&mut ShareHandles>) -> i32;

        fn gl_shared_image_new() -> UniquePtr<GlSharedImage>;

        fn cleanup(self: Pin<&mut GlSharedImage>);

        fn initialize(
            self: Pin<&mut GlSharedImage>,
            width: GLsizei,
            height: GLsizei,
            handle_id: u64,
            allocation_size: GLuint64,
            format: GlFormat,
            internal_format: GLenum,
        ) -> GLenum;

        fn import_from_handle(
            self: Pin<&mut GlSharedImage>,
            share_handles: UniquePtr<ShareHandles>,
            image_data: &SharedImageData,
        ) -> GLenum;

        fn get_image_data(self: &GlSharedImage) -> &SharedImageData;
        fn get_image_data_mut(self: Pin<&mut GlSharedImage>) -> &mut SharedImageData;
        fn get_texture_id(self: &GlSharedImage) -> GLuint;

        unsafe fn recv_image_blit_with_extents(
            self: Pin<&mut GlSharedImage>,
            src_texture_id: GLuint,
            src_texture_target: GLenum,
            src_dimensions: &ImageExtent,
            invert: bool,
            prev_fbo: GLuint,
        );

        unsafe fn recv_image_blit(
            self: Pin<&mut GlSharedImage>,
            src_texture_id: GLuint,
            src_texture_target: GLenum,
            invert: bool,
            prev_fbo: GLuint,
        );

        unsafe fn send_image_blit_with_extents(
            self: Pin<&mut GlSharedImage>,
            dst_texture_id: GLuint,
            dst_texture_target: GLenum,
            dst_dimensions: &ImageExtent,
            invert: bool,
            prev_fbo: GLuint,
        );

        unsafe fn send_image_blit(
            self: Pin<&mut GlSharedImage>,
            dst_texture_id: GLuint,
            dst_texture_target: GLenum,
            invert: bool,
            prev_fbo: GLuint,
        );

    }
}

impl SharedImageData {
    // Constants taken from gl.h
    pub const GL_TEXTURE_2D: GLenum = GLenum(0x0DE1);
    pub const GL_RGBA8: GLenum = GLenum(0x8058);

    pub fn from_shmem_img_data(data: &ShmemDataInternal) -> SharedImageData {
        SharedImageData {
            id: data.handle_id,
            width: data.width,
            height: data.height,
            format: GlFormat::from(data.format),
            allocation_size: data.allocation_size,
        }
    }
}

#[cfg(test)]
mod tests {
    use cxx::UniquePtr;

    use crate::opengl::gl_shared_image::ffi::gl_share_handles_new;

    use super::ffi::{gl_external_initialize, gl_shared_image_new, GlSharedImage};

    fn _init_gl_image() -> UniquePtr<GlSharedImage> {
        assert!(gl_external_initialize());
        gl_shared_image_new()
    }

    #[test]
    fn gl_shared_image_share_handles_new() {
        let share_handles = gl_share_handles_new();
        assert_eq!(share_handles.get_memory_handle(), -1);
    }

    #[test]
    fn gl_shared_image_create() {
        let _ = _init_gl_image();
    }

    #[test]
    fn gl_shared_image_cleanup() {
        let mut gl_shared_image = _init_gl_image();
        gl_shared_image.as_mut().unwrap().cleanup();
    }

    #[test]
    fn gl_shared_image_data() {
        let mut gl_shared_image = _init_gl_image();
        const TEST_VAL: u32 = 12345;

        {
            let sh_dat = gl_shared_image.as_mut().unwrap().get_image_data_mut();
            sh_dat.id = TEST_VAL;
        }

        {
            let sh_dat = gl_shared_image.get_image_data();
            assert_eq!(sh_dat.id, TEST_VAL);
        }
    }

    //     #[test]
    //     fn vk_shared_image_init() {
    //         let mut vk_setup = vk_setup_new();
    //         vk_setup.as_mut().unwrap().initialize_vulkan();

    //         let _instance = vk_setup.as_ref().unwrap().get_vk_instance();
    //         let device = vk_setup.as_ref().unwrap().get_vk_device();
    //         let physical_device = vk_setup.as_ref().unwrap().get_vk_physical_device();
    //         // let queue = vk_setup.as_ref().unwrap().get_vk_queue();

    //         // initialize_vulkan_handles(
    //         //     instance,
    //         //     vk_setup.as_ref().unwrap().get_vk_physical_device(),
    //         // );

    //         let mut vk_shared_image = vk_shared_image_new();
    //         vk_shared_image.as_mut().unwrap().initialize(
    //             device,
    //             physical_device,
    //             vk_setup.get_vk_queue(),
    //             vk_setup.get_vk_command_buffer(),
    //             1,
    //             2,
    //             VkFormat::VK_FORMAT_R8G8B8A8_UNORM,
    //             3,
    //         );

    //         assert_eq!(vk_shared_image.get_image_data().width, 1);
    //         assert_eq!(vk_shared_image.get_image_data().height, 2);
    //         assert_eq!(
    //             vk_shared_image.get_image_data().format,
    //             VkFormat::VK_FORMAT_R8G8B8A8_UNORM
    //         );
    //         assert_eq!(vk_shared_image.get_image_data().id, 3);

    //         let _ = vk_shared_image
    //             .as_mut()
    //             .unwrap()
    //             .export_handles(vk_setup.get_external_handle_info());
    //     }

    //     #[test]
    //     fn vk_shared_image_handle_exchange() {
    //         let vk_setup = _init_vulkan();

    //         let mut original_img = vk_shared_image_new();

    //         let width: u32 = 1;
    //         let height: u32 = 2;
    //         let format = VkFormat::VK_FORMAT_R8G8B8A8_UNORM;
    //         original_img.as_mut().unwrap().initialize(
    //             vk_setup.get_vk_device(),
    //             vk_setup.get_vk_physical_device(),
    //             vk_setup.get_vk_queue(),
    //             vk_setup.get_vk_command_buffer(),
    //             width,
    //             height,
    //             format,
    //             0,
    //         );

    //         let share_handles = original_img
    //             .as_mut()
    //             .unwrap()
    //             .export_handles(vk_setup.get_external_handle_info());

    //         let mut import_img = vk_shared_image_new();
    //         let image_data = original_img.get_image_data();
    //         import_img.as_mut().unwrap().import_from_handle(
    //             vk_setup.get_vk_device(),
    //             vk_setup.get_vk_physical_device(),
    //             share_handles,
    //             image_data,
    //         );
    //     }
}
