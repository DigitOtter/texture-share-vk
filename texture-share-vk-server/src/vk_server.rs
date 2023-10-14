use std::cell::RefCell;
use std::fs;
use std::io::{self, Error, ErrorKind};
use std::mem::ManuallyDrop;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime};
use texture_share_vk_base::cxx::UniquePtr;
use texture_share_vk_base::ipc::platform::img_data::ImgData;
use texture_share_vk_base::ipc::platform::ipc_commands::{
	CommFindImage, CommInitImage, CommandTag, ResultData, ResultFindImage, ResultInitImage,
	ResultMsg,
};
use texture_share_vk_base::ipc::platform::ShmemDataInternal;
use texture_share_vk_base::ipc::platform::{LockGuard, ReadLockGuard, Timeout};
use texture_share_vk_base::ipc::{IpcConnection, IpcShmem, IpcSocket};
use texture_share_vk_base::vk_setup::ffi::{vk_setup_new, VkSetup};
use texture_share_vk_base::vk_shared_image::ffi::{vk_shared_image_new, VkFormat, VkSharedImage};

pub(super) struct ImageData {
	pub ipc_info: IpcShmem,
	pub vk_shared_image: UniquePtr<VkSharedImage>,
}

pub struct VkServer {
	socket: Arc<Mutex<IpcSocket>>,
	socket_path: String,
	shmem_prefix: String,
	vk_setup: UniquePtr<VkSetup>,
	images: Vec<ImageData>,
}

impl Drop for VkServer {
	fn drop(&mut self) {
		// Ensure that images are cleared before vk_setup is destroyed
		self.images.clear();

		let _ = fs::remove_file(self.socket_path.to_owned());
	}
}

impl VkServer {
	const IPC_TIMEOUT: Duration = Duration::from_millis(5000);
	const NO_CONNECTION_TIMEOUT: Duration = Duration::from_millis(1000 * 1000);

	pub fn new(
		socket_path: &str,
		shmem_prefix: &str,
		connection_timeout: Duration,
	) -> Result<VkServer, Box<dyn std::error::Error>> {
		let _ = fs::remove_file(socket_path.to_owned());

		let socket = Arc::new(Mutex::new(
			IpcSocket::new(socket_path, connection_timeout).map_err(|e| Box::new(e))?,
		));

		let mut vk_setup = vk_setup_new();
		vk_setup.as_mut().unwrap().initialize_vulkan();

		let images = Vec::default();
		Ok(VkServer {
			socket,
			socket_path: socket_path.to_string(),
			shmem_prefix: shmem_prefix.to_string(),
			vk_setup,
			images,
		})
	}

	pub fn set_timeout(&mut self, connection_timeout: Duration) {
		self.socket.lock().unwrap().timeout = connection_timeout;
	}

	pub fn loop_server(
		mut self,
		stop_bit: Arc<AtomicBool>,
	) -> Result<(), Box<dyn std::error::Error>> {
		//stop_bit.store(false, Ordering::Relaxed);

		let listener_clone = self.socket.clone();
		let stop_clone = stop_bit.clone();
		let accept_thread_fcn = move || {
			while !stop_clone.load(Ordering::Relaxed) {
				VkServer::try_accept(&listener_clone)?;
			}

			Ok::<(), io::Error>(())
		};

		let accept_thread = thread::spawn(accept_thread_fcn);

		// Stop server if no connection was established after NO_CONNECTION_TIMEOUT
		let mut conn_timeout = SystemTime::now() + VkServer::NO_CONNECTION_TIMEOUT;

		let connections_clone = self.socket.clone().lock().unwrap().connections.clone();
		while !accept_thread.is_finished() && !stop_bit.load(Ordering::Relaxed) {
			let num_active_connections = VkServer::process_commands(
				connections_clone.clone(),
				self.vk_setup.as_ref().unwrap(),
				&self.shmem_prefix,
				&mut self.images,
			)?;

			println!("Active connections: {}", num_active_connections);

			// Stop if no connections active
			if num_active_connections == 0 {
				if SystemTime::now() > conn_timeout {
					println!("No connections active. Closing server...");
					break;
				}
			} else {
				conn_timeout = SystemTime::now() + VkServer::NO_CONNECTION_TIMEOUT;
			}
		}

		stop_bit.clone().store(true, Ordering::Relaxed);
		accept_thread.join().unwrap()?;

		Ok(())
	}

	fn try_accept(socket: &Arc<Mutex<IpcSocket>>) -> Result<Option<()>, Error> {
		let lock = socket.lock().unwrap();
		lock.try_accept().map(|c| match c {
			Some(_) => Some(()),
			None => None,
		})
	}

	fn poll_sockets(
		connections: Arc<Mutex<Vec<RefCell<IpcConnection>>>>,
		vk_setup: &VkSetup,
		shmem_prefix: &str,
		images: &mut Vec<ImageData>,
	) -> Result<usize, Box<dyn std::error::Error>> {
		Ok(0)
	}

	fn process_commands(
		connections: Arc<Mutex<Vec<RefCell<IpcConnection>>>>,
		vk_setup: &VkSetup,
		shmem_prefix: &str,
		images: &mut Vec<ImageData>,
	) -> Result<usize, Box<dyn std::error::Error>> {
		let mut connections_to_close = Vec::default();

		let mut lock = connections.lock();
		let mut conns = lock.as_mut();

		for i in 0..conns.as_ref().unwrap().len() {
			let conn = &conns.as_ref().unwrap()[i];

			// Try to receive command. If connection was closed by peer, remove this connection from vector
			let cmd = match conn.borrow_mut().recv_command_if_available() {
				Err(e) => match e.kind() {
					ErrorKind::BrokenPipe => {
						connections_to_close.push(i);
						continue;
					}
					_ => Err(e),
				},
				o => o,
			}?;

			if cmd.is_none() {
				continue;
			}

			let cmd = cmd.unwrap();
			let res = match cmd.tag {
				CommandTag::InitImage => {
					println!("Processing init message");
					VkServer::process_cmd_init_image(
						&conn.borrow(),
						unsafe { &cmd.data.init_img },
						vk_setup,
						shmem_prefix,
						images,
					)
				}
				CommandTag::FindImage => {
					println!("Processing find message");
					VkServer::process_cmd_find_image(
						&conn.borrow(),
						unsafe { &cmd.data.find_img },
						vk_setup,
						images,
					)
				}
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
						ErrorKind::BrokenPipe => {
							connections_to_close.push(i);
							continue;
						}
						_ => Err(e),
					},
				},
				s => s,
			}?
		}

		// Remove connections that were closed by peer
		for ci in connections_to_close.iter().rev() {
			conns.as_mut().unwrap().remove(*ci);
		}

		Ok(conns.as_ref().unwrap().len())
	}

	fn process_cmd_init_image(
		connection: &IpcConnection,
		cmd: &CommInitImage,
		vk_setup: &VkSetup,
		shmem_prefix: &str,
		images: &mut Vec<ImageData>,
	) -> Result<(), Box<dyn std::error::Error>> {
		let img_name_str = ImgData::convert_shmem_array_to_str(&cmd.image_name);

		let image_index = images.iter_mut().position(|it| {
			let rlock = it
				.ipc_info
				.acquire_rlock(Timeout::Val(VkServer::IPC_TIMEOUT))
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
			let mut handles = vk_shared_image
				.unwrap()
				.as_mut()
				.unwrap()
				.export_handles(vk_setup.get_external_handle_info());
			connection
				.send_anillary_handles(&[handles.as_mut().unwrap().release_memory_handle()])?;

			// Receive ack
			connection.recv_ack()?;
		}

		Ok(())
	}

	fn process_cmd_find_image(
		connection: &IpcConnection,
		cmd: &CommFindImage,
		vk_setup: &VkSetup,
		images: &mut Vec<ImageData>,
	) -> Result<(), Box<dyn std::error::Error>> {
		let img_name_str = ImgData::convert_shmem_array_to_str(&cmd.image_name);

		let image_and_lock: Option<(ImgData, &mut UniquePtr<VkSharedImage>, ReadLockGuard)> =
			images.iter_mut().find_map(|it| {
				let rlock = it
					.ipc_info
					.acquire_rlock(Timeout::Val(VkServer::IPC_TIMEOUT))
					.unwrap();
				let rdata = IpcShmem::acquire_rdata(&rlock);

				if ImgData::convert_shmem_array_to_str(&rdata.name)
					.cmp(&img_name_str)
					.is_eq()
				{
					Some((
						ImgData {
							name: rdata.name,
							shmem_name: ImgData::convert_shmem_str_to_array(it.ipc_info.get_name()),
							width: rdata.width,
							height: rdata.height,
							format: rdata.format,
							allocation_size: rdata.allocation_size,
						},
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
			let mut shared_handles = vk_shared_image
				.unwrap()
				.as_mut()
				.unwrap()
				.export_handles(vk_setup.get_external_handle_info());
			let fd = shared_handles.as_mut().unwrap().release_memory_handle();
			connection.send_anillary_handles(&[fd])?;
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
		image_vec: &'a mut Vec<ImageData>,
		image_name: &str,
		shmem_name: &str,
		image_index: Option<usize>,
	) -> Result<
		(
			ResultInitImage,
			Option<&'a mut UniquePtr<VkSharedImage>>,
			Option<LockGuard<'a>>,
		),
		Box<dyn std::error::Error>,
	> {
		// Check if an image with the given name is available
		let image: &mut ImageData = {
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
				let vk_shared_image = vk_shared_image_new();
				image_vec.push(ImageData {
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
			.acquire_lock(Timeout::Val(VkServer::IPC_TIMEOUT))
			.unwrap();
		let mut data = IpcShmem::acquire_data(&lock);

		// Update VkSharedImage
		image.vk_shared_image.as_mut().unwrap().initialize(
			vk_setup.get_vk_device(),
			vk_setup.get_vk_physical_device(),
			vk_setup.get_vk_queue(),
			vk_setup.get_vk_command_buffer(),
			cmd.width,
			cmd.height,
			VkFormat::from(cmd.format),
			data.handle_id + 1,
		);

		// Update Shmem data
		VkServer::update_shmem_data(&mut data, &image.vk_shared_image);

		// Generate ResultMsg data
		let img_data = ImgData {
			name: data.name,
			shmem_name: ImgData::convert_shmem_str_to_array(image.ipc_info.get_name()),
			width: data.width,
			height: data.height,
			format: data.format,
			allocation_size: data.allocation_size,
		};

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
		shmem_data.format = vk_data.format.into();
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

	const TIMEOUT: Duration = Duration::from_millis(2000);
	const SOCKET_PATH: &str = "test_socket.sock";
	const SHMEM_PREFIX: &str = "shared_images_";

	fn _server_create() -> VkServer {
		VkServer::new(SOCKET_PATH, SHMEM_PREFIX, TIMEOUT).unwrap()
	}

	#[test]
	fn server_create() {
		let _ = VkServer::new(SOCKET_PATH, SHMEM_PREFIX, TIMEOUT).unwrap();
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

		let conn = IpcConnection::try_connect(SOCKET_PATH, TIMEOUT).unwrap();
		assert!(conn.is_some());

		stop_bit.store(true, Ordering::Relaxed);

		server_thread.join().unwrap();
	}
}
