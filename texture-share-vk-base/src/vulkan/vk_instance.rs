use ash::{vk, Instance};
use std::ffi::CStr;

use super::vk_entry::VkEntry;

pub struct VkInstance {
	entry: Box<VkEntry>,
	pub instance: Instance,
	import_only: bool,
}

impl Drop for VkInstance {
	fn drop(&mut self) {
		if !self.import_only {
			unsafe { self.instance.destroy_instance(None) };
		}
	}
}

impl VkInstance {
	pub fn new(
		entry: Option<Box<VkEntry>>,
		instance_name: &CStr,
	) -> Result<VkInstance, vk::Result> {
		const ENABLE_VALIDATION: bool = true;
		let validation_layers: &[&CStr] =
			&[CStr::from_bytes_with_nul(b"VK_LAYER_KHRONOS_validation\0").unwrap()];

		let entry = entry.unwrap_or(Box::new(VkEntry::new()?));

		let mut extensions = vec![
			vk::KhrGetPhysicalDeviceProperties2Fn::name().as_ptr(),
			vk::KhrExternalSemaphoreCapabilitiesFn::name().as_ptr(),
			vk::KhrExternalMemoryCapabilitiesFn::name().as_ptr(),
		];

		if ENABLE_VALIDATION {
			extensions.push(vk::ExtDebugUtilsFn::name().as_ptr())
		}

		let layers = if ENABLE_VALIDATION && entry.check_layer_support(validation_layers) {
			validation_layers
				.iter()
				.map(|&x| x.as_ptr())
				.collect::<Vec<_>>()
		} else {
			println!("Validation layers not supported!");
			Vec::default()
		};

		let app_info = vk::ApplicationInfo::builder()
			.api_version(vk::make_api_version(0, 1, 2, 0))
			.application_name(instance_name)
			.build();

		let create_info = vk::InstanceCreateInfo::builder()
			.application_info(&app_info)
			.enabled_extension_names(&extensions)
			.enabled_layer_names(&layers)
			.build();

		let instance = unsafe { entry.entry.create_instance(&create_info, None) }?;
		Ok(VkInstance {
			entry,
			instance,
			import_only: false,
		})
	}

	pub fn import_vk(
		entry: Option<Box<VkEntry>>,
		vk_instance: vk::Instance,
		import_only: bool,
	) -> Result<VkInstance, vk::Result> {
		let entry = entry.unwrap_or(Box::new(VkEntry::new()?));
		let instance = unsafe { Instance::load(entry.entry.static_fn(), vk_instance) };

		Ok(VkInstance {
			entry,
			instance,
			import_only,
		})
	}

	pub fn get_memory_type(
		&self,
		vk_physical_device: vk::PhysicalDevice,
		memory_type_bits_requirement: u32,
		required_properties: vk::MemoryPropertyFlags,
	) -> Option<u32> {
		let memory_properties = unsafe {
			self.instance
				.get_physical_device_memory_properties(vk_physical_device)
		};

		// Code taken from https://registry.khronos.org/vulkan/specs/1.3/html/chap11.html#memory-device
		for memory_index in 0..memory_properties.memory_type_count {
			let memory_type_bits = 1 << memory_index;

			// Check bits
			if (memory_type_bits_requirement & memory_type_bits) != 0 {
				let properties =
					memory_properties.memory_types[memory_index as usize].property_flags;
				let has_required_properties =
					(properties & required_properties) == required_properties;
				if has_required_properties {
					return Some(memory_index);
				}
			}
		}

		None
	}
}

#[cfg(test)]
mod tests {
	use std::ffi::CStr;

	use super::VkInstance;

	#[test]
	fn vk_instance_new() {
		let _vk_instance =
			VkInstance::new(None, CStr::from_bytes_with_nul(b"VkInstace\0").unwrap())
				.expect("Failed to create VkInstance");
	}
}
