use std::{
	borrow::Cow,
	ffi::{c_char, c_int, CStr},
	ptr::{self, null_mut, NonNull},
	time::Duration,
};
use texture_share_vk_base::{
	ash::vk,
	bindings::vk_setup_from_c,
	ipc::platform::{img_data::ImgFormat, ReadLockGuard, ShmemDataInternal},
	vk_setup::VkSetup,
};

use crate::VkClient;

type VkFence = vk::Fence;
type VkOffset3D = vk::Offset3D;
type VkImage = vk::Image;
type VkImageLayout = vk::ImageLayout;

#[repr(C)]
enum ImageLookupResult {
	Error = -1,
	NotFound = 0,
	Found = 1,
	RequiresUpdate = 2,
}

fn get_str<'a>(buf: &'a *const c_char) -> Cow<'a, str> {
	unsafe { CStr::from_ptr(buf.to_owned()) }.to_string_lossy()
}

//#[repr(transparent)]
//struct ClientImageData(ShmemDataInternal);

struct ClientImageDataGuard<'a> {
	_read_lock: ReadLockGuard<'a>,
	image_data: &'a ShmemDataInternal,
}

impl<'a> ClientImageDataGuard<'a> {
	#[no_mangle]
	extern "C" fn get_image_data(&self) -> &ShmemDataInternal {
		&self.image_data
	}
}

#[no_mangle]
extern "C" fn vk_client_new(
	socket_path: *const c_char,
	vk_setup: Option<NonNull<VkSetup>>,
	timeout_in_millis: u64,
) -> *mut VkClient {
	let vk_setup = match vk_setup {
		Some(ptr) => unsafe { vk_setup_from_c(ptr.as_ptr()) },
		None => Box::new(
			VkSetup::new(CStr::from_bytes_with_nul(b"VkClient\0").unwrap())
				.map_err(|_x| {
					println!("Failed to create VkSetup");
					return ptr::null_mut::<VkClient>();
				})
				.unwrap(),
		),
	};

	let vk_client = VkClient::new(
		&get_str(&socket_path),
		vk_setup,
		Duration::from_millis(timeout_in_millis),
	);

	match vk_client {
		Err(e) => {
			println!("Failed to create VkClient with error '{:}'", e);
			return null_mut();
		}
		Ok(s) => Box::into_raw(Box::new(s)),
	}
}

#[no_mangle]
extern "C" fn vk_client_new_with_server_launch(
	socket_path: *const c_char,
	vk_setup: Option<NonNull<VkSetup>>,
	client_timeout_in_millis: u64,
	server_program: *const c_char,
	server_lock_path: *const c_char,
	server_socket_path: *const c_char,
	shmem_prefix: *const c_char,
	server_socket_timeout_in_millis: u64,
	server_connection_wait_timeout_in_millis: u64,
	server_ipc_timeout_in_millis: u64,
	server_lockfile_timeout_in_millis: u64,
	server_spawn_timeout_in_millis: u64,
) -> *mut VkClient {
	let vk_setup = match vk_setup {
		Some(ptr) => unsafe { vk_setup_from_c(ptr.as_ptr()) },
		None => Box::new(
			VkSetup::new(CStr::from_bytes_with_nul(b"VkClient\0").unwrap())
				.map_err(|_x| {
					println!("Failed to create VkSetup");
					return ptr::null_mut::<VkClient>();
				})
				.unwrap(),
		),
	};

	let vk_client = VkClient::new_with_server_launch(
		&get_str(&socket_path),
		vk_setup,
		Duration::from_millis(client_timeout_in_millis),
		&get_str(&server_program),
		&get_str(&server_lock_path),
		&get_str(&server_socket_path),
		&get_str(&shmem_prefix),
		Duration::from_millis(server_socket_timeout_in_millis),
		Duration::from_millis(server_connection_wait_timeout_in_millis),
		Duration::from_millis(server_ipc_timeout_in_millis),
		Duration::from_millis(server_lockfile_timeout_in_millis),
		Duration::from_millis(server_spawn_timeout_in_millis),
	);

	match vk_client {
		Err(e) => {
			println!("Failed to create VkClient with error '{:}'", e);
			return null_mut();
		}
		Ok(s) => Box::into_raw(Box::new(s)),
	}
}

#[no_mangle]
extern "C" fn vk_client_destroy(vk_client: Option<NonNull<VkClient>>) {
	if vk_client.is_none() {
		return;
	}

	let vk_client = vk_client.unwrap().as_ptr();
	drop(unsafe { Box::from_raw(vk_client) });
}

#[no_mangle]
extern "C" fn vk_client_init_image(
	vk_client: *mut VkClient,
	image_name: *const c_char,
	width: u32,
	height: u32,
	format: ImgFormat,
	overwrite_existing: bool,
) -> ImageLookupResult {
	match unsafe { vk_client.as_mut() }.unwrap().init_image(
		&get_str(&image_name),
		width,
		height,
		format,
		overwrite_existing,
	) {
		Ok(Some(true)) => return ImageLookupResult::RequiresUpdate,
		Ok(Some(false)) => return ImageLookupResult::Found,
		Ok(None) => return ImageLookupResult::NotFound,
		Err(e) => {
			println!("Failed to init image with err '{:}'", e);
			return ImageLookupResult::Error;
		}
	}
}

#[no_mangle]
extern "C" fn vk_client_find_image(
	vk_client: *mut VkClient,
	image_name: *const c_char,
	force_update: bool,
) -> ImageLookupResult {
	let local_image = unsafe { vk_client.as_mut() }
		.unwrap()
		.find_image(&get_str(&image_name), force_update);

	match local_image {
		Ok(Some(true)) => return ImageLookupResult::RequiresUpdate,
		Ok(Some(false)) => return ImageLookupResult::Found,
		Ok(None) => return ImageLookupResult::NotFound,
		Err(e) => {
			println!("Failed to find image with err '{:}'", e);
			return ImageLookupResult::Error;
		}
	}
}

#[no_mangle]
extern "C" fn vk_client_find_image_data<'a>(
	vk_client: *mut VkClient,
	image_name: *const c_char,
	force_update: bool,
) -> *mut ClientImageDataGuard<'a> {
	let local_image = unsafe { vk_client.as_mut() }
		.unwrap()
		.find_image_data(&get_str(&image_name), force_update);

	match local_image {
		Ok(Some(d)) => {
			return Box::into_raw(Box::new(ClientImageDataGuard {
				_read_lock: d.0,
				image_data: d.1,
			}))
		}
		Ok(None) => return null_mut(),
		Err(e) => {
			println!("Failed to find image with error '{:}'", e);
			return null_mut();
		}
	}
}

#[no_mangle]
extern "C" fn vk_client_image_data_guard_read<'a>(
	image_data_guard: *const ClientImageDataGuard<'a>,
) -> &'a ShmemDataInternal {
	&unsafe { image_data_guard.as_ref() }.unwrap().image_data
}

#[no_mangle]
extern "C" fn vk_client_image_data_guard_destroy(
	image_data_guard: Option<NonNull<ClientImageDataGuard>>,
) {
	if image_data_guard.is_none() {
		return;
	}

	let image_data_guard = image_data_guard.unwrap().as_ptr();
	drop(unsafe { Box::from_raw(image_data_guard) });
}

#[no_mangle]
extern "C" fn vk_client_send_image(
	vk_client: *mut VkClient,
	image_name: *const c_char,
	image: VkImage,
	orig_layout: VkImageLayout,
	target_layout: VkImageLayout,
	fence: VkFence,
	extents: Option<NonNull<VkOffset3D>>,
) -> c_int {
	let vk_client = unsafe { vk_client.as_mut() }.unwrap();
	let image_name = &get_str(&image_name);

	let res = match extents {
		None => vk_client.send_image(image_name, image, orig_layout, target_layout, fence),
		Some(s) => unsafe {
			vk_client.send_image_with_extents_unchecked(
				image_name,
				image,
				orig_layout,
				target_layout,
				fence,
				s.as_ptr(),
			)
		},
	};

	match res {
		Ok(Some(_)) => return 1,
		Ok(None) => return 0,
		Err(e) => {
			println!("Failed to send image with error '{:}'", e);
			return -1;
		}
	}
}

#[no_mangle]
extern "C" fn vk_client_recv_image(
	vk_client: *mut VkClient,
	image_name: *const c_char,
	image: VkImage,
	orig_layout: VkImageLayout,
	target_layout: VkImageLayout,
	fence: VkFence,
	extents: Option<NonNull<VkOffset3D>>,
) -> c_int {
	let vk_client = unsafe { vk_client.as_mut() }.unwrap();
	let image_name = &get_str(&image_name);

	let res = match extents {
		None => vk_client.recv_image(image_name, image, orig_layout, target_layout, fence),
		Some(s) => unsafe {
			vk_client.recv_image_with_extents_unchecked(
				image_name,
				image,
				orig_layout,
				target_layout,
				fence,
				s.as_ptr(),
			)
		},
	};

	match res {
		Ok(Some(_)) => return 1,
		Ok(None) => return 0,
		Err(e) => {
			println!("Failed to send image with error '{:}'", e);
			return -1;
		}
	}
}
