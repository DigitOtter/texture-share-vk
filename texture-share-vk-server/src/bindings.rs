use std::{
	borrow::Cow,
	ffi::{c_char, c_int, c_uint, CStr},
	ptr::{null_mut, NonNull},
	sync::{atomic::AtomicBool, Arc},
	time::Duration,
};

use texture_share_vk_base::vk_device::VkPhysicalDeviceOptions;

use crate::VkServer;

//type c_str = [c_char; 1024];

fn get_str<'a>(buf: &'a *const c_char) -> Cow<'a, str> {
	unsafe { CStr::from_ptr(buf.to_owned()) }.to_string_lossy()
}

// unsafe fn vk_server_as_mut<'a>(vk_server: &'a *mut VkServer) -> Pin<&'a mut VkServer> {
//     unsafe { Pin::new_unchecked(vk_server.as_mut().unwrap()) }
// }

#[no_mangle]
extern "C" fn vk_server_new(
	socket_path: *const c_char,
	shmem_prefix: *const c_char,
	socket_timeout_in_millis: u64,
	no_connection_timeout_in_millis: u64,
	ipc_timeout_in_millis: u64,
	gpu_vendor_id: Option<NonNull<u32>>,
	gpu_device_id: Option<NonNull<u32>>,
	gpu_device_name: Option<NonNull<c_char>>,
) -> *mut VkServer {
	let socket_path = get_str(&socket_path);
	let shmem_prefix = get_str(&shmem_prefix);

	let physical_device_options = VkPhysicalDeviceOptions {
		vendor_id: gpu_vendor_id.map(|x| unsafe { *x.as_ref() }),
		device_id: gpu_device_id.map(|x| unsafe { *x.as_ref() }),
		device_name: gpu_device_name.map(|x| unsafe { CStr::from_ptr(x.as_ptr()).to_owned() }),
		..Default::default()
	};

	match VkServer::new(
		&socket_path,
		&shmem_prefix,
		Duration::from_millis(socket_timeout_in_millis),
		Duration::from_millis(no_connection_timeout_in_millis),
		Duration::from_millis(ipc_timeout_in_millis),
		Some(physical_device_options),
	) {
		Err(_) => null_mut(),
		Ok(s) => Box::into_raw(Box::new(s)),
	}
}

#[no_mangle]
extern "C" fn vk_server_destroy(vk_server: Option<NonNull<VkServer>>) {
	match vk_server {
		None => {}
		Some(s) => drop(unsafe { Box::from_raw(s.as_ptr()) }),
	};
}

#[no_mangle]
extern "C" fn vk_server_loop(vk_server: *mut *mut VkServer) -> c_int {
	let stop_bit = Arc::new(AtomicBool::new(false));

	let vk_server = unsafe { Box::from_raw(*vk_server) };
	let res = vk_server.loop_server(stop_bit);
	match res {
		Err(e) => {
			println!("Server loop encountered error: '{:}'", e);
			return -1;
		}
		Ok(_) => {
			return 0;
		}
	}
}
