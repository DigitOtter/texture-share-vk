use std::ffi::CStr;
use std::fs;
use std::io::{Error, ErrorKind};
use std::mem::ManuallyDrop;

use std::os::fd::IntoRawFd;
use std::time::Duration;
use texture_share_vk_base::ash::vk;
use texture_share_vk_base::ipc::platform::img_data::ImgData;
use texture_share_vk_base::ipc::platform::ipc_commands::{
	CommFindImage, CommInitImage, CommandTag, ResultData, ResultFindImage, ResultInitImage,
	ResultMsg,
};
use texture_share_vk_base::ipc::platform::ShmemDataInternal;
use texture_share_vk_base::ipc::platform::{LockGuard, ReadLockGuard, Timeout};
use texture_share_vk_base::ipc::{IpcConnection, IpcShmem, IpcSocket};
use texture_share_vk_base::vk_device::{VkDevice, VkPhysicalDeviceOptions};
use texture_share_vk_base::vk_instance::VkInstance;
use texture_share_vk_base::vk_setup::VkSetup;
use texture_share_vk_base::vk_shared_image::VkSharedImage;

pub(super) struct ServerImageData {
	pub ipc_info: IpcShmem,
	pub vk_shared_image: VkSharedImage,
}

pub struct VkServer {
	pub(crate) socket: IpcSocket,
	pub(crate) socket_path: String,
	pub(crate) shmem_prefix: String,
	pub(crate) vk_setup: VkSetup,
	pub(crate) images: Vec<ServerImageData>,
	pub(crate) connection_wait_timeout: Duration,
	pub(crate) ipc_timeout: Duration,
}

impl Drop for VkServer {
	fn drop(&mut self) {
		// Ensure that images are cleared before vk_setup is destroyed
		self.images
			.drain(..)
			.for_each(|x| x.vk_shared_image.destroy(&self.vk_setup.device));

		let _ = fs::remove_file(self.socket_path.to_owned());
	}
}

impl VkServer {
	pub(crate) const LISTENER_EVENT_KEY: usize = usize::MAX - 1;

	pub fn new(
		socket_path: &str,
		shmem_prefix: &str,
		socket_timeout: Duration,
		connection_wait_timeout: Duration,
		ipc_timeout: Duration,
		physical_device_options: Option<VkPhysicalDeviceOptions>,
	) -> Result<VkServer, Box<dyn std::error::Error>> {
		let _ = fs::remove_file(socket_path.to_owned());

		let socket = IpcSocket::new(socket_path, socket_timeout).map_err(|e| Box::new(e))?;

		let vk_instance = VkInstance::new(None, CStr::from_bytes_with_nul(b"VkServer\0").unwrap())?;
		let vk_device = VkDevice::new(&vk_instance, physical_device_options)?;
		let vk_setup = VkSetup::new(vk_instance, vk_device);

		let images = Vec::default();
		Ok(VkServer {
			socket,
			socket_path: socket_path.to_string(),
			shmem_prefix: shmem_prefix.to_string(),
			vk_setup,
			images,
			connection_wait_timeout,
			ipc_timeout,
		})
	}

	pub fn set_timeout(&mut self, connection_timeout: Duration) {
		self.socket.timeout = connection_timeout;
	}

	pub(crate) fn process_single_connection(
		conn: &IpcConnection,
		vk_setup: &VkSetup,
		shmem_prefix: &str,
		images: &mut Vec<ServerImageData>,
		ipc_timeout: Duration,
	) -> Result<bool, Box<dyn std::error::Error>> {
		// Try to receive command. If connection was closed by peer, remove this connection from vector
		let cmd = match conn.recv_command_if_available() {
			Err(e) => match e.kind() {
				ErrorKind::BrokenPipe => {
					return Ok(false);
				}
				_ => Err(e),
			},
			o => o,
		}?;

		if cmd.is_none() {
			return Ok(true);
		}

		let cmd = cmd.unwrap();
		let res = match cmd.tag {
			CommandTag::InitImage => VkServer::process_cmd_init_image(
				conn,
				unsafe { &cmd.data.init_img },
				vk_setup,
				shmem_prefix,
				images,
				ipc_timeout,
			),
			CommandTag::FindImage => VkServer::process_cmd_find_image(
				conn,
				unsafe { &cmd.data.find_img },
				vk_setup,
				images,
				ipc_timeout,
			),
			// CommandTag::RenameImage => Server::process_cmd_rename_image(
			//     &conn.borrow(),
			//     unsafe { &cmd.data.rename_img },
			//     vk_setup,
			//     images,
			// ),
			#[allow(unreachable_patterns)]
			_ => Err::<(), Box<dyn std::error::Error>>(Box::new(Error::new(
				ErrorKind::InvalidData,
				"Unknown command received",
			))),
		};

		match res {
			Err(e) => match e.downcast_ref::<Error>() {
				None => Err(e),
				Some(ioe) => match ioe.kind() {
					ErrorKind::BrokenPipe => return Ok(false),
					_ => Err(e),
				},
			},
			s => s,
		}?;

		Ok(true)
	}

	fn process_cmd_init_image(
		connection: &IpcConnection,
		cmd: &CommInitImage,
		vk_setup: &VkSetup,
		shmem_prefix: &str,
		images: &mut Vec<ServerImageData>,
		ipc_timeout: Duration,
	) -> Result<(), Box<dyn std::error::Error>> {
		let img_name_str = ImgData::convert_shmem_array_to_str(&cmd.image_name);

		let image_index = images.iter_mut().position(|it| {
			let rlock = it
				.ipc_info
				.acquire_rlock(Timeout::Val(ipc_timeout))
				.unwrap();
			let rdata = IpcShmem::acquire_rdata(&rlock);
			ImgData::convert_shmem_array_to_str(&rdata.name)
				.cmp(&img_name_str)
				.is_eq()
		});

		// Update image, keep lock
		let shmem_name = shmem_prefix.to_owned() + &img_name_str;
		let (result_msg_data, vk_shared_image, _lock) = VkServer::update_shared_image(
			cmd,
			vk_setup,
			images,
			&img_name_str,
			&shmem_name,
			image_index,
			ipc_timeout,
		)?;

		// Send result to client
		let res_msg = ResultMsg {
			tag: CommandTag::InitImage,
			data: ResultData {
				init_img: ManuallyDrop::new(result_msg_data),
			},
		};
		connection.send_result(res_msg)?;

		// Send shared handles if image was created
		if vk_shared_image.is_some() {
			let handles = vk_shared_image.unwrap().export_handle(&vk_setup.device)?;
			connection.send_anillary_handles(&[handles.into_raw_fd()])?;

			// Receive ack
			connection.recv_ack()?;
		}

		Ok(())
	}

	fn process_cmd_find_image(
		connection: &IpcConnection,
		cmd: &CommFindImage,
		vk_setup: &VkSetup,
		images: &mut Vec<ServerImageData>,
		ipc_timeout: Duration,
	) -> Result<(), Box<dyn std::error::Error>> {
		let img_name_str = ImgData::convert_shmem_array_to_str(&cmd.image_name);

		let image_and_lock: Option<(ImgData, &mut VkSharedImage, ReadLockGuard)> =
			images.iter_mut().find_map(|it| {
				let rlock = it
					.ipc_info
					.acquire_rlock(Timeout::Val(ipc_timeout))
					.unwrap();
				let rdata = IpcShmem::acquire_rdata(&rlock);

				if ImgData::convert_shmem_array_to_str(&rdata.name)
					.cmp(&img_name_str)
					.is_eq()
				{
					Some((
						ImgData::from_shmem_data_internal(
							ImgData::convert_shmem_str_to_array(it.ipc_info.get_name()),
							rdata.clone(),
						),
						&mut it.vk_shared_image,
						rlock,
					))
				} else {
					None
				}
			});

		// Keep lock, extract image
		let (image, vk_shared_image, _opt_lock) = match image_and_lock {
			Some((image, vk_img, lock)) => (Some(image), Some(vk_img), Some(lock)),
			_ => (None, None, None),
		};

		// Send result message
		let res_data = match image {
			Some(img_data) => ResultFindImage {
				image_found: true,
				img_data,
			},
			None => ResultFindImage {
				image_found: false,
				img_data: ImgData::default(),
			},
		};

		connection.send_result(ResultMsg {
			tag: CommandTag::FindImage,
			data: ResultData {
				find_img: ManuallyDrop::new(res_data),
			},
		})?;

		if vk_shared_image.is_some() {
			let fd = vk_shared_image.unwrap().export_handle(&vk_setup.device)?;
			connection.send_anillary_handles(&[fd.into_raw_fd()])?;
			connection.recv_ack()?;
		}

		Ok(())
	}

	// fn process_cmd_rename_image(
	//     connection: &IpcConnection,
	//     cmd: &CommRenameImage,
	//     vk_setup: &VkSetup,
	//     images: &mut Vec<ImageData>,
	// ) {
	// }

	fn update_shared_image<'a>(
		cmd: &CommInitImage,
		vk_setup: &VkSetup,
		image_vec: &'a mut Vec<ServerImageData>,
		image_name: &str,
		shmem_name: &str,
		image_index: Option<usize>,
		ipc_timeout: Duration,
	) -> Result<
		(
			ResultInitImage,
			Option<&'a mut VkSharedImage>,
			Option<LockGuard<'a>>,
		),
		Box<dyn std::error::Error>,
	> {
		// Check if an image with the given name is available
		let image: &mut ServerImageData = {
			if image_index.is_some() {
				// Only overwrite image if explicitly requested
				if !cmd.overwrite_existing {
					return Ok((
						ResultInitImage {
							image_created: false,
							img_data: ImgData::default(),
						},
						None,
						None,
					));
				}

				image_vec.get_mut(image_index.unwrap()).unwrap()
			} else {
				let ipc_info = IpcShmem::new(shmem_name, image_name, true)?;
				let vk_shared_image = VkSharedImage::new(
					&vk_setup.instance,
					&vk_setup.device,
					1,
					1,
					vk::Format::R8G8B8A8_UNORM,
					0,
				)?;
				image_vec.push(ServerImageData {
					ipc_info,
					vk_shared_image,
				});
				image_vec.last_mut().unwrap()
			}
		};

		// Update VkShared image and Shmem data
		// Lock access
		let lock = image
			.ipc_info
			.acquire_lock(Timeout::Val(ipc_timeout))
			.unwrap();
		let mut data = IpcShmem::acquire_data(&lock);

		// Update VkSharedImage
		image.vk_shared_image.resize_image(
			&vk_setup.instance,
			&vk_setup.device,
			cmd.width,
			cmd.height,
			VkSharedImage::get_vk_format(cmd.format),
			data.handle_id + 1,
		)?;

		// Update Shmem data
		VkServer::update_shmem_data(&mut data, &image.vk_shared_image);

		// Generate ResultMsg data
		let img_data = ImgData::from_shmem_data_internal(
			ImgData::convert_shmem_str_to_array(image.ipc_info.get_name()),
			data.clone(),
		);

		// Return result, vk_shared_img, and lock
		return Ok((
			ResultInitImage {
				image_created: true,
				img_data,
			},
			Some(&mut image.vk_shared_image),
			Some(lock),
		));
	}

	fn update_shmem_data(shmem_data: &mut ShmemDataInternal, vk_shared_image: &VkSharedImage) {
		let vk_data = vk_shared_image.get_image_data();

		shmem_data.width = vk_data.width;
		shmem_data.height = vk_data.height;
		shmem_data.format = VkSharedImage::get_img_format(vk_data.format);
		shmem_data.allocation_size = vk_data.allocation_size;
		shmem_data.handle_id = vk_data.id;
	}
}

#[cfg(test)]
mod tests {
	use std::{fs, thread};
	use std::{
		sync::{
			atomic::{AtomicBool, Ordering},
			Arc,
		},
		thread::spawn,
		time::Duration,
	};

	use texture_share_vk_base::ipc::IpcConnection;

	use super::VkServer;

	const SOCKET_TIMEOUT: Duration = Duration::from_millis(2000);
	const NO_CONNECTION_TIMEOUT: Duration = Duration::from_millis(2000);
	const IPC_TIMEOUT: Duration = Duration::from_millis(2000);
	const SOCKET_PATH: &str = "test_socket.sock";
	const SHMEM_PREFIX: &str = "shared_images_";

	fn _server_create() -> VkServer {
		VkServer::new(
			SOCKET_PATH,
			SHMEM_PREFIX,
			SOCKET_TIMEOUT,
			NO_CONNECTION_TIMEOUT,
			IPC_TIMEOUT,
			None,
		)
		.unwrap()
	}

	#[test]
	fn server_create() {
		let _ = VkServer::new(
			SOCKET_PATH,
			SHMEM_PREFIX,
			SOCKET_TIMEOUT,
			NO_CONNECTION_TIMEOUT,
			IPC_TIMEOUT,
			None,
		)
		.unwrap();
	}

	#[test]
	fn server_loop() {
		let _ = fs::remove_file(SOCKET_PATH);
		let stop_bit = Arc::new(AtomicBool::new(false));

		let stop_clone = stop_bit.clone();
		let server_thread = spawn(move || {
			let server = _server_create();
			server.loop_server(stop_clone).expect("Server loop failed")
		});

		thread::sleep(Duration::from_secs(1));
		assert_eq!(server_thread.is_finished(), false);

		stop_bit.store(true, Ordering::Relaxed);

		server_thread.join().unwrap();
	}

	#[test]
	fn server_accept() {
		let _ = fs::remove_file(SOCKET_PATH);
		let stop_bit = Arc::new(AtomicBool::new(false));

		let stop_clone = stop_bit.clone();
		let server_thread = spawn(move || {
			let server = _server_create();
			server.loop_server(stop_clone).expect("Server loop failed")
		});

		let conn = IpcConnection::try_connect(SOCKET_PATH, SOCKET_TIMEOUT).unwrap();
		assert!(conn.is_some());

		stop_bit.store(true, Ordering::Relaxed);

		server_thread.join().unwrap();
	}
}
