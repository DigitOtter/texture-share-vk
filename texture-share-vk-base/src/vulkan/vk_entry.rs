use std::ffi::CStr;

use ash::{vk, Entry};

pub struct VkEntry {
	pub entry: Entry,
}

impl VkEntry {
	pub fn new() -> Result<VkEntry, vk::Result> {
		let entry = unsafe { Entry::load() }.map_err(|_| vk::Result::NOT_READY)?;
		Ok(VkEntry { entry })
	}

	pub(crate) unsafe fn to_cstr(buf: &[i8]) -> &CStr {
		let buf: &[u8] = &*(buf as *const [i8] as *const [u8]);
		let cstr = CStr::from_bytes_until_nul(buf).unwrap();
		cstr
	}

	pub fn check_layer_support(&self, layers: &[&CStr]) -> bool {
		let props = self
			.entry
			.enumerate_instance_layer_properties()
			.map_err(|_| return false)
			.unwrap();

		let mut all_available = true;
		layers.iter().for_each(|&l| {
			if props
				.iter()
				.any(|p| unsafe { Self::to_cstr(&p.layer_name) } == l)
				== false
			{
				all_available = false;
			}
		});

		all_available
	}
}
