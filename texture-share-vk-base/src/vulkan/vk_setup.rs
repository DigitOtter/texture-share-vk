use std::mem::ManuallyDrop;

use super::vk_device::VkDevice;
use super::vk_instance::VkInstance;

pub struct VkSetup {
	// Use ManuallyDrop to ensure that VkInstance is dropped AFTER VkDevice
	pub instance: ManuallyDrop<VkInstance>,
	pub device: ManuallyDrop<VkDevice>,
}

impl VkSetup {
	pub fn new(vk_instance: VkInstance, vk_device: VkDevice) -> VkSetup {
		VkSetup {
			instance: ManuallyDrop::new(vk_instance),
			device: ManuallyDrop::new(vk_device),
		}
	}
}

impl Drop for VkSetup {
	fn drop(&mut self) {
		unsafe {
			ManuallyDrop::drop(&mut self.device);
			ManuallyDrop::drop(&mut self.instance);
		}
	}
}

#[cfg(test)]
mod tests {
	use std::ffi::CStr;

	use super::VkSetup;
	use crate::{vk_device::VkDevice, vk_instance::VkInstance};

	#[test]
	fn vk_setup_new() {
		let vk_instance = VkInstance::new(
			None,
			CStr::from_bytes_with_nul(b"VkInstance\0").unwrap(),
			true,
		)
		.unwrap();
		let vk_device = VkDevice::new(&vk_instance, None).unwrap();
		let _vk_setup = VkSetup::new(vk_instance, vk_device);
	}
}
