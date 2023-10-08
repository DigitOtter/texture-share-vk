use std::pin::Pin;

use cxx::UniquePtr;

use crate::vk_setup::{
    ffi::{self, VkSetup},
    VkDevice, VkInstance, VkPhysicalDevice, VkQueue,
};

unsafe fn vk_setup_as_mut<'a>(vk_setup: &'a *mut VkSetup) -> Pin<&'a mut VkSetup> {
    unsafe { Pin::new_unchecked(vk_setup.as_mut().unwrap()) }
}

#[no_mangle]
extern "C" fn vk_setup_new() -> *mut VkSetup {
    return ffi::vk_setup_new().into_raw();
}

#[no_mangle]
extern "C" fn vk_setup_destroy(vk_setup: *mut VkSetup) {
    unsafe { UniquePtr::from_raw(vk_setup) };
}

#[no_mangle]
extern "C" fn vk_setup_initialize_vulkan(vk_setup: *mut VkSetup) {
    unsafe { vk_setup_as_mut(&vk_setup) }.initialize_vulkan();
}

#[no_mangle]
extern "C" fn vk_setup_import_vulkan(
    vk_setup: *mut VkSetup,
    instance: VkInstance,
    device: VkDevice,
    physical_device: VkPhysicalDevice,
    graphics_queue: VkQueue,
    graphics_queue_index: u32,
    import_only: bool,
) {
    let vk_setup = unsafe { vk_setup_as_mut(&vk_setup) };
    if import_only {
        vk_setup.import_vulkan(
            instance,
            device,
            physical_device,
            graphics_queue,
            graphics_queue_index,
        );
    } else {
        vk_setup.import_vulkan_as_owned(
            instance,
            device,
            physical_device,
            graphics_queue,
            graphics_queue_index,
        );
    }
}

#[no_mangle]
extern "C" fn vk_setup_cleanup_vulkan(vk_setup: *mut VkSetup) {
    unsafe { vk_setup_as_mut(&vk_setup) }.cleanup_vulkan();
}

#[no_mangle]
extern "C" fn vk_setup_is_vulkan_initialized(vk_setup: *mut VkSetup) {
    unsafe { vk_setup.as_ref().unwrap() }.is_vulkan_initialized();
}
