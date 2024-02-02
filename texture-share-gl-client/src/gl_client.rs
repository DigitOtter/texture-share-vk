use std::collections::HashMap;
use texture_share_ipc::platform::daemon_launch::server_connect_and_daemon_launch;
use texture_share_ipc::platform::{ReadLockGuard, Timeout};

use std::io::{Error, ErrorKind};
use std::{mem::ManuallyDrop, os::fd::OwnedFd, time::Duration};

use texture_share_ipc::platform::img_data::{ImgData, ImgFormat};
use texture_share_ipc::platform::ipc_commands::{
	CommFindImage, CommInitImage, CommandData, CommandMsg, CommandTag,
};
use texture_share_ipc::platform::ShmemDataInternal;
use texture_share_ipc::{IpcConnection, IpcShmem};

use crate::gl_shared_image::{GlImageExtent, GlSharedImage};
use crate::opengl::glad;

pub struct ImageData {
	pub ipc_info: IpcShmem,
	pub vk_shared_image: GlSharedImage,
}

pub struct GlClient {
	connection: IpcConnection,
	shared_images: HashMap<String, ImageData>,
	//timeout: Duration,
}

impl Drop for GlClient {
	fn drop(&mut self) {
		// Ensure that images are cleared before destroying vulkan instance
		self.shared_images.clear();
	}
}

impl GlClient {
	const IPC_TIMEOUT: Duration = Duration::from_millis(5000);

	pub fn initialize_gl_external() -> bool {
		match GlSharedImage::init_gl() {
			Ok(_) => true,
			Err(_) => false,
		}
	}

	pub fn new(socket_path: &str, timeout: Duration) -> Result<GlClient, Error> {
		let connection = IpcConnection::try_connect(socket_path, timeout)?;
		if connection.is_none() {
			return Err(Error::new(
				ErrorKind::Interrupted,
				format!("Connection to '{}' timed out", socket_path),
			));
		}

		let shared_images = HashMap::default();

		Ok(GlClient {
			connection: connection.unwrap(),
			shared_images,
			//timeout,
		})
	}

	pub fn new_with_server_launch(
		socket_path: &str,
		client_timeout: Duration,
		server_program: &str,
		server_lock_path: &str,
		server_socket_path: &str,
		shmem_prefix: &str,
		server_socket_timeout: Duration,
		server_connection_wait_timeout: Duration,
		server_ipc_timeout: Duration,
		server_lockfile_timeout: Duration,
		server_spawn_timeout: Duration,
	) -> Result<GlClient, Error> {
		let conn_fn = || {
			let connection = match IpcConnection::try_connect(socket_path, client_timeout) {
				Err(e) => match e.kind() {
					ErrorKind::ConnectionRefused => Ok(None),
					_ => Err(e),
				},
				s => s,
			}?;
			if connection.is_none() {
				return Ok(None);
			}

			let shared_images = HashMap::default();

			Ok(Some(GlClient {
				connection: connection.unwrap(),
				shared_images,
				//timeout,
			}))
		};

		let res = server_connect_and_daemon_launch(
			server_program,
			server_lock_path,
			server_socket_path,
			shmem_prefix,
			server_socket_timeout,
			server_connection_wait_timeout,
			server_ipc_timeout,
			server_lockfile_timeout,
			server_spawn_timeout,
			&conn_fn,
		)?;

		if let Some(client) = res {
			return Ok(client);
		} else {
			return Err(Error::new(
				ErrorKind::Interrupted,
				format!("Connection to '{}' timed out", socket_path),
			));
		}
	}

	fn check_for_update(image_data: &ImageData) -> bool {
		image_data.ipc_info.get_id_unchecked() != image_data.vk_shared_image.get_data().id
	}

	pub fn init_image(
		&mut self,
		image_name: &str,
		width: u32,
		height: u32,
		format: ImgFormat,
		overwrite_existing: bool,
	) -> Result<Option<bool>, Box<dyn std::error::Error>> {
		let image_name_buf = ImgData::convert_shmem_str_to_array(image_name);
		let cmd_msg = CommandMsg {
			tag: CommandTag::InitImage,
			data: CommandData {
				init_img: ManuallyDrop::new(CommInitImage {
					image_name: image_name_buf,
					shmem_name: image_name_buf,
					width,
					height,
					format,
					overwrite_existing,
					..Default::default() // TODO: Add GPU vendor and device id
				}),
			},
		};

		self.connection.send_command(cmd_msg)?;

		// Receive message and check for validity
		let res_msg = self.connection.recv_result()?;
		let res_data: Option<&ImgData> = match &res_msg {
			None => Ok(None),
			Some(msg) => match msg.tag {
				CommandTag::InitImage => {
					let data = unsafe { &msg.data.init_img };
					if data.image_created {
						Ok(Some(&data.img_data))
					} else {
						Ok(None)
					}
				}
				_ => Err(Box::new(Error::new(
					ErrorKind::InvalidData,
					"Received invalid data from server",
				))),
			},
		}?;

		// Don't import image if not created
		if res_data.is_none() {
			return Ok(None);
		}

		let res_data = res_data.unwrap();

		let mut share_handles = self.connection.recv_ancillary(1)?;

		self.connection.send_ack()?;

		let res = self.add_new_image(&res_data, &mut share_handles)?;

		let res = match res {
			Some(r) => Some(GlClient::check_for_update(r)),
			None => None,
		};
		Ok(res)
	}

	pub fn find_image(
		&mut self,
		image_name: &str,
		force_update: bool,
	) -> Result<Option<bool>, Box<dyn std::error::Error>> {
		let res = self.find_image_internal(image_name, force_update)?;
		let res = match res {
			Some(r) => Some(GlClient::check_for_update(r)),
			None => None,
		};
		Ok(res)
	}

	pub fn find_image_data(
		&mut self,
		image_name: &str,
		force_update: bool,
	) -> Result<Option<(ReadLockGuard, &ShmemDataInternal)>, Box<dyn std::error::Error>> {
		let res = self.find_image_internal(image_name, force_update)?;
		let res = match res {
			Some(image_data) => {
				let rlock: ReadLockGuard = image_data
					.ipc_info
					.acquire_rlock(Timeout::Val(GlClient::IPC_TIMEOUT))?;
				let rdata = IpcShmem::acquire_rdata(&rlock);
				Some((rlock, rdata))
			}
			None => None,
		};
		Ok(res)
	}

	pub fn send_image(
		&mut self,
		image_name: &str,
		src_texture_id: glad::GLuint,
		src_texture_target: glad::GLenum,
		invert: bool,
		prev_fbo: glad::GLuint,
	) -> Result<Option<()>, Box<dyn std::error::Error>> {
		let remote_image = self.shared_images.get_mut(image_name);
		if remote_image.is_none() {
			return Ok(None);
		}

		let remote_image = remote_image.unwrap();
		// recv_image_... is correct, as it's from the perspective of the remove image
		remote_image
			.vk_shared_image
			.recv_blit_image(
				src_texture_id,
				src_texture_target,
				&GlImageExtent {
					top_left: [0, 0],
					bottom_right: [
						remote_image.vk_shared_image.get_data().width as i32,
						remote_image.vk_shared_image.get_data().height as i32,
					],
				},
				invert,
				prev_fbo,
			)
			.unwrap();

		Ok(Some(()))
	}

	pub fn send_image_with_extents(
		&mut self,
		image_name: &str,
		src_texture_id: glad::GLuint,
		src_texture_target: glad::GLenum,
		invert: bool,
		prev_fbo: glad::GLuint,
		extent: &GlImageExtent,
	) -> Result<Option<()>, Box<dyn std::error::Error>> {
		let remote_image = self.shared_images.get_mut(image_name);
		if remote_image.is_none() {
			return Ok(None);
		}

		let remote_image = remote_image.unwrap();
		// recv_image_... is correct, as it's from the perspective of the remove image
		remote_image
			.vk_shared_image
			.recv_blit_image(src_texture_id, src_texture_target, extent, invert, prev_fbo)
			.unwrap();
		Ok(Some(()))
	}

	pub fn recv_image(
		&mut self,
		image_name: &str,
		dst_texture_id: glad::GLuint,
		dst_texture_target: glad::GLenum,
		invert: bool,
		prev_fbo: glad::GLuint,
	) -> Result<Option<()>, Box<dyn std::error::Error>> {
		let remote_image = self.shared_images.get_mut(image_name);
		if remote_image.is_none() {
			return Ok(None);
		}

		let remote_image = remote_image.unwrap();
		// send_image_... is correct, as it's from the perspective of the remove image
		remote_image
			.vk_shared_image
			.send_blit_image(
				dst_texture_id,
				dst_texture_target,
				&GlImageExtent {
					top_left: [0, 0],
					bottom_right: [
						remote_image.vk_shared_image.get_data().width as i32,
						remote_image.vk_shared_image.get_data().height as i32,
					],
				},
				invert,
				prev_fbo,
			)
			.unwrap();

		Ok(Some(()))
	}

	pub fn recv_image_with_extents(
		&mut self,
		image_name: &str,
		dst_texture_id: glad::GLuint,
		dst_texture_target: glad::GLenum,
		invert: bool,
		prev_fbo: glad::GLuint,
		extent: &GlImageExtent,
	) -> Result<Option<()>, Box<dyn std::error::Error>> {
		let remote_image = self.shared_images.get_mut(image_name);
		if remote_image.is_none() {
			return Ok(None);
		}

		let remote_image = remote_image.unwrap();
		// send_image_... is correct, as it's from the perspective of the remove image
		remote_image
			.vk_shared_image
			.send_blit_image(dst_texture_id, dst_texture_target, extent, invert, prev_fbo)
			.map_err(|x| {
				Box::new(std::io::Error::new(
					ErrorKind::InvalidData,
					format!("GL Error: {}", x),
				))
			})?;
		Ok(Some(()))
	}

	fn add_new_image(
		&mut self,
		img_data: &ImgData,
		share_handles: &mut Vec<OwnedFd>,
	) -> Result<Option<&ImageData>, Box<dyn std::error::Error>> {
		// TODO: Update if sharing more handles
		debug_assert_eq!(share_handles.len(), 1);
		let fd = share_handles.pop().unwrap();

		let image_name = ImgData::convert_shmem_array_to_str(&img_data.data.name);
		let image_data = self.create_local_image(img_data, fd)?;
		self.shared_images
			.insert(image_name.to_string(), image_data);

		Ok(Some(self.shared_images.get(&image_name).unwrap()))
	}

	fn create_local_image(
		&self,
		img_data: &ImgData,
		img_mem_fd: OwnedFd,
	) -> Result<ImageData, Box<dyn std::error::Error>> {
		let shmem = IpcShmem::new(
			&ImgData::convert_shmem_array_to_str(&img_data.shmem_name),
			&ImgData::convert_shmem_array_to_str(&img_data.data.name),
			false,
		)?;

		let vk_shared_image = {
			let rlock = shmem.acquire_rlock(Timeout::Val(GlClient::IPC_TIMEOUT))?;
			let _rdata = IpcShmem::acquire_rdata(&rlock);

			let vk_shared_image = GlSharedImage::import_handle(
				img_mem_fd,
				img_data.data.width as i32,
				img_data.data.height as i32,
				img_data.data.allocation_size,
				GlSharedImage::get_gl_format(img_data.data.format),
				GlSharedImage::get_gl_internal_format(img_data.data.format) as u32,
				img_data.data.handle_id,
			)
			.unwrap();
			vk_shared_image
		};

		Ok(ImageData {
			ipc_info: shmem,
			vk_shared_image,
		})
	}

	fn find_image_internal(
		&mut self,
		image_name: &str,
		force_update: bool,
	) -> Result<Option<&ImageData>, Box<dyn std::error::Error>> {
		if force_update {
			let res = self.find_image_cmd(image_name)?;
			return Ok(res);
		}

		let res = match self.shared_images.contains_key(image_name) {
			true => self.shared_images.get(image_name),
			false => self.find_image_cmd(image_name)?,
		};

		Ok(res)
	}

	fn find_image_cmd(
		&mut self,
		image_name: &str,
	) -> Result<Option<&ImageData>, Box<dyn std::error::Error>> {
		let cmd_dat = ManuallyDrop::new(CommFindImage {
			image_name: ImgData::convert_shmem_str_to_array(image_name),
		});
		let cmd_msg = CommandMsg {
			tag: CommandTag::FindImage,
			data: CommandData { find_img: cmd_dat },
		};
		self.connection.send_command(cmd_msg)?;

		let res_msg = self.connection.recv_result()?;
		let res_data: Option<&ImgData> = match &res_msg {
			None => Ok(None),
			Some(msg) => match msg.tag {
				CommandTag::FindImage => {
					let data = unsafe { &msg.data.find_img };
					if data.image_found {
						Ok(Some(&data.img_data))
					} else {
						Ok(None)
					}
				}
				_ => Err(Box::new(Error::new(
					ErrorKind::InvalidData,
					"Received invalid data from server",
				))),
			},
		}?;

		if res_data.is_none() {
			return Ok(None);
		}

		let res_data = res_data.unwrap();

		let mut share_handles = self.connection.recv_ancillary(1)?;

		self.connection.send_ack()?;

		let fd = share_handles.pop().unwrap();

		let image_data = self.create_local_image(&res_data, fd)?;
		let local_image = self.shared_images.get_mut(image_name);
		if local_image.is_none() {
			self.shared_images
				.insert(image_name.to_string(), image_data);
		} else {
			*(local_image.unwrap()) = image_data;
		};

		Ok(Some(&self.shared_images.get(image_name).unwrap()))
	}
}

#[cfg(test)]
mod tests {
	use std::time::Duration;
	use std::{fs, thread};

	use texture_share_ipc::IpcSocket;

	use super::GlClient;

	const TIMEOUT: Duration = Duration::from_millis(2000);
	const SOCKET_PATH: &str = "test_socket.sock";

	fn _create_server_socket() -> IpcSocket {
		IpcSocket::new(SOCKET_PATH, TIMEOUT).unwrap()
	}

	#[test]
	fn gl_client_create() {
		let _ = fs::remove_file(SOCKET_PATH);

		let server_socket_fcn = || {
			let server_socket = _create_server_socket();
			server_socket.try_accept().unwrap()
		};

		let server_thread = thread::spawn(server_socket_fcn);

		let _client = GlClient::new(SOCKET_PATH, TIMEOUT).unwrap();

		let server_res = server_thread.join().unwrap();
		assert!(server_res.is_some());
	}
}
