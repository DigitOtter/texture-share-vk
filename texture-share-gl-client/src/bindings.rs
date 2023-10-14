use std::{
	borrow::Cow,
	ffi::CStr,
	ptr::{null_mut, NonNull},
	time::Duration,
};

use cxx::{type_id, ExternType};
use libc::{c_char, c_int};
use texture_share_ipc::platform::{img_data::ImgFormat, ReadLockGuard, ShmemDataInternal};

use crate::{
	gl_shared_image::{GLenum, GLsizei, GLuint},
	GlClient,
};

#[repr(C)]
enum ImageLookupResult {
	Error = -1,
	NotFound = 0,
	Found = 1,
	RequiresUpdate = 2,
}

#[repr(C)]
pub struct ImageExtent {
	top_left: [GLsizei; 2],
	bottom_right: [GLsizei; 2],
}

unsafe impl ExternType for ImageExtent {
	type Id = type_id!("opengl::ImageExtent");
	type Kind = cxx::kind::Trivial;
}

fn get_str<'a>(buf: &'a *const c_char) -> Cow<'a, str> {
	unsafe { CStr::from_ptr(buf.to_owned()) }.to_string_lossy()
}

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
extern "C" fn gl_client_initialize_external_gl() -> bool {
	GlClient::initialize_gl_external()
}

#[no_mangle]
extern "C" fn gl_client_new(socket_path: *const c_char, timeout_in_millis: u64) -> *mut GlClient {
	let gl_client = GlClient::new(
		&get_str(&socket_path),
		Duration::from_millis(timeout_in_millis),
	);

	match gl_client {
		Err(e) => {
			println!("Failed to create GlClient with error '{:}'", e);
			return null_mut();
		}
		Ok(s) => Box::into_raw(Box::new(s)),
	}
}

#[no_mangle]
extern "C" fn gl_client_new_with_server_launch(
	socket_path: *const c_char,
	client_timeout_in_millis: u64,
	server_program: *const c_char,
	server_lock_path: *const c_char,
	server_socket_path: *const c_char,
	shmem_prefix: *const c_char,
	server_connection_timeout_in_millia: u64,
	server_spawn_timeout_in_millis: u64,
) -> *mut GlClient {
	let gl_client = GlClient::new_with_server_launch(
		&get_str(&socket_path),
		Duration::from_millis(client_timeout_in_millis),
		&get_str(&server_program),
		&get_str(&server_lock_path),
		&get_str(&server_socket_path),
		&get_str(&shmem_prefix),
		Duration::from_millis(server_connection_timeout_in_millia),
		Duration::from_millis(server_spawn_timeout_in_millis),
	);

	match gl_client {
		Err(e) => {
			println!("Failed to create GlClient with error '{:}'", e);
			return null_mut();
		}
		Ok(s) => Box::into_raw(Box::new(s)),
	}
}

#[no_mangle]
extern "C" fn gl_client_destroy(gl_client: Option<NonNull<GlClient>>) {
	if gl_client.is_none() {
		return;
	}

	let gl_client = gl_client.unwrap().as_ptr();
	drop(unsafe { Box::from_raw(gl_client) });
}

#[no_mangle]
extern "C" fn gl_client_init_image(
	gl_client: *mut GlClient,
	image_name: *const c_char,
	width: u32,
	height: u32,
	format: ImgFormat,
	overwrite_existing: bool,
) -> ImageLookupResult {
	match unsafe { gl_client.as_mut() }.unwrap().init_image(
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
extern "C" fn gl_client_find_image(
	gl_client: *mut GlClient,
	image_name: *const c_char,
	force_update: bool,
) -> ImageLookupResult {
	let local_image = unsafe { gl_client.as_mut() }
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
extern "C" fn gl_client_find_image_data<'a>(
	gl_client: *mut GlClient,
	image_name: *const c_char,
	force_update: bool,
) -> *mut ClientImageDataGuard<'a> {
	let local_image = unsafe { gl_client.as_mut() }
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
extern "C" fn gl_client_image_data_guard_read<'a>(
	image_data_guard: *const ClientImageDataGuard<'a>,
) -> &'a ShmemDataInternal {
	&unsafe { image_data_guard.as_ref() }.unwrap().image_data
}

#[no_mangle]
extern "C" fn gl_client_image_data_guard_destroy(
	image_data_guard: Option<NonNull<ClientImageDataGuard>>,
) {
	if image_data_guard.is_none() {
		return;
	}

	let image_data_guard = image_data_guard.unwrap().as_ptr();
	drop(unsafe { Box::from_raw(image_data_guard) });
}

#[no_mangle]
extern "C" fn gl_client_send_image(
	gl_client: *mut GlClient,
	image_name: *const c_char,
	src_texture_id: GLuint,
	src_texture_target: GLenum,
	invert: bool,
	prev_fbo: GLuint,
	extents: *const ImageExtent,
) -> c_int {
	let image_name = &get_str(&image_name);
	let gl_client = unsafe { gl_client.as_mut().unwrap() };

	let res = match extents.is_null() {
		true => gl_client.send_image(
			image_name,
			src_texture_id,
			src_texture_target,
			invert,
			prev_fbo,
		),
		false => unsafe {
			gl_client.send_image_with_extents(
				image_name,
				src_texture_id,
				src_texture_target,
				invert,
				prev_fbo,
				extents.as_ref().unwrap(),
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
extern "C" fn gl_client_recv_image(
	gl_client: *mut GlClient,
	image_name: *const c_char,
	dst_texture_id: GLuint,
	dst_texture_target: GLenum,
	invert: bool,
	prev_fbo: GLuint,
	extents: *const ImageExtent,
) -> c_int {
	let gl_client = unsafe { gl_client.as_mut() }.unwrap();
	let image_name = &get_str(&image_name);

	let res = match extents.is_null() {
		true => gl_client.recv_image(
			image_name,
			dst_texture_id,
			dst_texture_target,
			invert,
			prev_fbo,
		),
		false => unsafe {
			gl_client.recv_image_with_extents(
				image_name,
				dst_texture_id,
				dst_texture_target,
				invert,
				prev_fbo,
				extents.as_ref().unwrap(),
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
