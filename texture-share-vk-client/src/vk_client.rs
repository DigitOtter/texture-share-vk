use std::collections::HashMap;
use std::io::{Error, ErrorKind};
use std::{mem::ManuallyDrop, os::fd::OwnedFd, time::Duration};

use texture_share_vk_base::ash::vk;
use texture_share_vk_base::ipc::platform::daemon_launch::server_connect_and_daemon_launch;
use texture_share_vk_base::ipc::platform::img_data::{ImgData, ImgFormat};
use texture_share_vk_base::ipc::platform::ipc_commands::{
	CommFindImage, CommInitImage, CommandData, CommandMsg, CommandTag,
};
use texture_share_vk_base::ipc::platform::ShmemDataInternal;
use texture_share_vk_base::ipc::platform::{ReadLockGuard, Timeout};
use texture_share_vk_base::ipc::{IpcConnection, IpcShmem};

use texture_share_vk_base::vk_device::VkDevice;
use texture_share_vk_base::vk_setup::VkSetup;
use texture_share_vk_base::vk_shared_image::VkSharedImage;
use texture_share_vk_base::vk_shared_image::{ImageBlit, SharedImageData};

pub struct ImageData {
	pub ipc_info: IpcShmem,
	pub vk_shared_image: VkSharedImage,
}

pub struct VkClient {
	connection: IpcConnection,
	vk_setup: Box<VkSetup>,
	shared_images: HashMap<String, ImageData>,
	gpu_device_uuid: u128,
}

impl Drop for VkClient {
	fn drop(&mut self) {
		// Ensure that images are cleared before destroying vulkan instance
		self.shared_images
			.drain()
			.for_each(|x| x.1.vk_shared_image.destroy(&self.vk_setup.device));
	}
}

impl VkClient {
	const IPC_TIMEOUT: Duration = Duration::from_millis(5000);

	pub fn new(
		socket_path: &str,
		vk_setup: Box<VkSetup>,
		timeout: Duration,
	) -> Result<VkClient, Error> {
		let connection = IpcConnection::try_connect(socket_path, timeout)?;
		if connection.is_none() {
			return Err(Error::new(
				ErrorKind::Interrupted,
				format!("Connection to '{}' timed out", socket_path),
			));
		}

		let shared_images = HashMap::default();

		let gpu_device_uuid = VkDevice::get_gpu_device_uuid(
			&vk_setup.instance.instance,
			vk_setup.device.physical_device,
		)
		.as_u128();

		Ok(VkClient {
			connection: connection.unwrap(),
			vk_setup,
			shared_images,
			gpu_device_uuid,
		})
	}

	pub fn new_with_server_launch(
		socket_path: &str,
		vk_setup: Box<VkSetup>,
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
	) -> Result<VkClient, Error> {
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

			Ok(Some(connection.unwrap()))
		};

		let gpu_device_uuid = VkDevice::get_gpu_device_uuid(
			&vk_setup.instance.instance,
			vk_setup.device.physical_device,
		);

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
			Some(gpu_device_uuid),
			&conn_fn,
		)?;

		if let Some(connection) = res {
			return Ok(VkClient {
				connection,
				vk_setup,
				shared_images: HashMap::default(),
				gpu_device_uuid: gpu_device_uuid.as_u128(),
			});
		} else {
			return Err(Error::new(
				ErrorKind::Interrupted,
				format!("Connection to '{}' timed out", socket_path),
			));
		}
	}

	pub fn get_vk_setup(&self) -> &VkSetup {
		&self.vk_setup
	}

	pub fn get_vk_setup_mut(&mut self) -> &mut VkSetup {
		&mut self.vk_setup
	}

	fn is_update_available(image_data: &ImageData) -> bool {
		image_data.ipc_info.get_id_unchecked() != image_data.vk_shared_image.get_image_data().id
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
					gpu_device_uuid: self.gpu_device_uuid,
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
			Some(r) => Some(VkClient::is_update_available(r)),
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
			Some(r) => Some(VkClient::is_update_available(r)),
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
					.acquire_rlock(Timeout::Val(VkClient::IPC_TIMEOUT))?;
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
		image: vk::Image,
		orig_layout: vk::ImageLayout,
		target_layout: vk::ImageLayout,
		fence: vk::Fence,
	) -> Result<Option<()>, Box<dyn std::error::Error>> {
		let remote_image = self.shared_images.get_mut(image_name);
		if remote_image.is_none() {
			return Ok(None);
		}

		// Send image
		let remote_image = remote_image.unwrap();
		remote_image.vk_shared_image.recv_image_blit(
			&self.vk_setup.device,
			&image,
			orig_layout,
			target_layout,
			fence,
		)?;

		Ok(Some(()))
	}

	pub fn send_image_with_extents(
		&mut self,
		image_name: &str,
		image: vk::Image,
		orig_layout: vk::ImageLayout,
		target_layout: vk::ImageLayout,
		fence: vk::Fence,
		extents: &[vk::Offset3D; 2],
	) -> Result<Option<()>, Box<dyn std::error::Error>> {
		unsafe {
			self.send_image_with_extents_unchecked(
				image_name,
				image,
				orig_layout,
				target_layout,
				fence,
				extents.as_ptr(),
			)
		}
	}

	pub(crate) unsafe fn send_image_with_extents_unchecked(
		&mut self,
		image_name: &str,
		image: vk::Image,
		orig_layout: vk::ImageLayout,
		target_layout: vk::ImageLayout,
		fence: vk::Fence,
		extents: *const vk::Offset3D,
	) -> Result<Option<()>, Box<dyn std::error::Error>> {
		let remote_image = self.shared_images.get_mut(image_name);
		if remote_image.is_none() {
			return Ok(None);
		}

		let extents: &[vk::Offset3D; 2] = unsafe {
			std::slice::from_raw_parts(extents, 2)
				.try_into()
				.unwrap_unchecked()
		};

		let remote_image = remote_image.unwrap();
		remote_image.vk_shared_image.recv_image_blit_with_extents(
			&self.vk_setup.device,
			&image,
			orig_layout,
			target_layout,
			extents,
			fence,
		)?;
		Ok(Some(()))
	}

	pub fn recv_image(
		&mut self,
		image_name: &str,
		image: vk::Image,
		orig_layout: vk::ImageLayout,
		target_layout: vk::ImageLayout,
		fence: vk::Fence,
	) -> Result<Option<()>, Box<dyn std::error::Error>> {
		let remote_image = self.shared_images.get_mut(image_name);
		if remote_image.is_none() {
			return Ok(None);
		}

		let remote_image = remote_image.unwrap();
		remote_image.vk_shared_image.send_image_blit(
			&self.vk_setup.device,
			&image,
			orig_layout,
			target_layout,
			fence,
		)?;

		Ok(Some(()))
	}

	pub fn recv_image_with_extents(
		&mut self,
		image_name: &str,
		image: vk::Image,
		orig_layout: vk::ImageLayout,
		target_layout: vk::ImageLayout,
		fence: vk::Fence,
		extents: &[vk::Offset3D; 2],
	) -> Result<Option<()>, Box<dyn std::error::Error>> {
		unsafe {
			self.recv_image_with_extents_unchecked(
				image_name,
				image,
				orig_layout,
				target_layout,
				fence,
				extents.as_ptr(),
			)
		}
	}

	pub(crate) unsafe fn recv_image_with_extents_unchecked(
		&mut self,
		image_name: &str,
		image: vk::Image,
		orig_layout: vk::ImageLayout,
		target_layout: vk::ImageLayout,
		fence: vk::Fence,
		extents: *const vk::Offset3D,
	) -> Result<Option<()>, Box<dyn std::error::Error>> {
		let remote_image = self.shared_images.get_mut(image_name);
		if remote_image.is_none() {
			return Ok(None);
		}

		let extents: &[vk::Offset3D; 2] = unsafe {
			std::slice::from_raw_parts(extents, 2)
				.try_into()
				.unwrap_unchecked()
		};

		let remote_image = remote_image.unwrap();
		remote_image.vk_shared_image.send_image_blit_with_extents(
			&self.vk_setup.device,
			&image,
			orig_layout,
			target_layout,
			extents,
			fence,
		)?;
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
		let image_data = Self::create_local_image(&self.vk_setup, img_data, fd)?;
		self.shared_images
			.insert(image_name.to_string(), image_data)
			.map(|x| x.vk_shared_image.destroy(&self.vk_setup.device));

		Ok(Some(self.shared_images.get(&image_name).unwrap()))
	}

	fn create_local_image(
		vk_setup: &VkSetup,
		img_data: &ImgData,
		img_mem_fd: OwnedFd,
	) -> Result<ImageData, Box<dyn std::error::Error>> {
		let shmem = IpcShmem::new(
			&ImgData::convert_shmem_array_to_str(&img_data.shmem_name),
			&ImgData::convert_shmem_array_to_str(&img_data.data.name),
			false,
		)?;

		let vk_shared_image = {
			let rlock = shmem.acquire_rlock(Timeout::Val(VkClient::IPC_TIMEOUT))?;
			let _rdata = IpcShmem::acquire_rdata(&rlock);

			let vk_shared_image = VkSharedImage::import_from_handle(
				&vk_setup.instance,
				&vk_setup.device,
				img_mem_fd,
				SharedImageData::from_shmem_img_data(&img_data.data),
			)?;
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
			gpu_device_uuid: self.gpu_device_uuid,
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

		let image_data = Self::create_local_image(&self.vk_setup, &res_data, fd)?;
		self.shared_images
			.insert(image_name.to_string(), image_data)
			.map(|x| x.vk_shared_image.destroy(&self.vk_setup.device));

		Ok(Some(&self.shared_images.get(image_name).unwrap()))
	}
}

#[cfg(test)]
mod tests {
	use std::ffi::CStr;
	use std::time::Duration;
	use std::{fs, thread};

	use texture_share_vk_base::ipc::IpcSocket;
	use texture_share_vk_base::vk_device::{self, VkDevice};
	use texture_share_vk_base::vk_instance::VkInstance;
	use texture_share_vk_base::vk_setup::VkSetup;

	use super::VkClient;

	const TIMEOUT: Duration = Duration::from_millis(2000);
	const SOCKET_PATH: &str = "test_socket.sock";

	fn _create_server_socket() -> IpcSocket {
		IpcSocket::new(SOCKET_PATH, TIMEOUT).unwrap()
	}

	#[test]
	fn client_create() {
		let _ = fs::remove_file(SOCKET_PATH);

		let server_socket_fcn = || {
			let server_socket = _create_server_socket();
			server_socket.try_accept().unwrap()
		};

		let server_thread = thread::spawn(server_socket_fcn);

		let vk_instance =
			VkInstance::new(None, CStr::from_bytes_with_nul(b"vk_setup\0").unwrap()).unwrap();
		let vk_device = VkDevice::new(&vk_instance, None).unwrap();
		let vk_setup = Box::new(VkSetup::new(vk_instance, vk_device));

		let _client = VkClient::new(SOCKET_PATH, vk_setup, TIMEOUT).unwrap();

		let server_res = server_thread.join().unwrap();
		assert!(server_res.is_some());
	}
}
