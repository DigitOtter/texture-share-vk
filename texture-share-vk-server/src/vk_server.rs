mod vk_copy_images;

use std::borrow::{Borrow, BorrowMut};
use std::cell::{RefCell, RefMut};
use std::collections::hash_map::{Entry, OccupiedEntry};
use std::collections::{hash_map, HashMap};
use std::ffi::{c_void, CStr, CString};
use std::fs;
use std::io::{Error, ErrorKind};
use std::mem::{ManuallyDrop, MaybeUninit};
use std::{alloc, ptr};

use std::os::fd::IntoRawFd;
use std::time::Duration;
use texture_share_vk_base::ipc::platform::img_data::ImgData;
use texture_share_vk_base::ipc::platform::ipc_commands::{
	CommCopyImage, CommFindImage, CommInitImage, CommandTag, ResultData, ResultFindImage,
	ResultInitImage, ResultMsg,
};
use texture_share_vk_base::ipc::platform::ShmemDataInternal;
use texture_share_vk_base::ipc::platform::{LockGuard, ReadLockGuard, Timeout};
use texture_share_vk_base::ipc::{IpcConnection, IpcShmem, IpcSocket};
use texture_share_vk_base::vk_cpu_shared_image::{AlignedRamBuffer, VkCpuSharedImage};
use texture_share_vk_base::vk_device::{self, VkDevice, VkPhysicalDeviceOptions};
use texture_share_vk_base::vk_instance::VkInstance;
use texture_share_vk_base::vk_setup::VkSetup;
use texture_share_vk_base::vk_shared_image::{self, VkSharedImage};
use texture_share_vk_base::{ash::vk, uuid};

use self::vk_copy_images::VkCopyImages;

pub(super) struct ServerImageData {
	pub ipc_info: IpcShmem,
	pub vk_shared_image: VkCpuSharedImage,
}

#[derive(Default)]
pub(super) struct GpuImageData {
	pub images: GpuImagesMap,
	pub ram_buffer: AlignedRamBuffer,
}

type DevicesMap = HashMap<u128, VkDevice>;

type GpuImagesMap = HashMap<u128, ServerImageData>;
type NameImagesMap = HashMap<String, GpuImageData>;

pub struct VkServer {
	pub(crate) socket: IpcSocket,
	pub(crate) socket_path: String,
	pub(crate) shmem_prefix: String,
	pub(crate) images: NameImagesMap,
	pub(crate) vk_instance: VkInstance,
	pub(crate) vk_devices: DevicesMap,
	pub(crate) connection_wait_timeout: Duration,
	pub(crate) ipc_timeout: Duration,
}

impl Drop for VkServer {
	fn drop(&mut self) {
		// Ensure that images are cleared before vk_devices are destroyed
		self.images.drain().for_each(|mut map| {
			map.1.images.drain().for_each(|x| {
				let _rlock =
					x.1.ipc_info
						.acquire_rlock(Timeout::Val(self.ipc_timeout))
						.expect("Failed to acquire lock on IpcData");
				let uuid = uuid::Uuid::from_u128(x.0);
				x.1.vk_shared_image.destroy(
					&self
						.vk_devices
						.get(&uuid.as_u128())
						.expect("Failed to find device for VkSharedImage"),
				);
			})
		});

		// Destroy devices before vk_instance
		self.vk_devices.clear();

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

		let gpu_device_uuid =
			VkDevice::get_gpu_device_uuid(&vk_instance.instance, vk_device.physical_device);

		let mut vk_devices = HashMap::default();
		vk_devices.insert(gpu_device_uuid.as_u128(), vk_device);

		let images = HashMap::default();

		Ok(VkServer {
			socket,
			socket_path: socket_path.to_string(),
			shmem_prefix: shmem_prefix.to_string(),
			images,
			vk_instance,
			vk_devices,
			connection_wait_timeout,
			ipc_timeout,
		})
	}

	pub fn set_timeout(&mut self, connection_timeout: Duration) {
		self.socket.timeout = connection_timeout;
	}

	pub(crate) fn process_single_connection(
		conn: &IpcConnection,
		vk_instance: &VkInstance,
		vk_devices: &mut DevicesMap,
		shmem_prefix: &str,
		images: &mut NameImagesMap,
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
				vk_instance,
				vk_devices,
				shmem_prefix,
				images,
				ipc_timeout,
			),
			CommandTag::FindImage => VkServer::process_cmd_find_image(
				conn,
				unsafe { &cmd.data.find_img },
				vk_instance,
				vk_devices,
				images,
				ipc_timeout,
			),
			CommandTag::CopyImage => VkServer::process_cmd_copy_image(
				conn,
				unsafe { &cmd.data.copy_img },
				vk_instance,
				vk_devices,
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
		vk_instance: &VkInstance,
		vk_devices: &mut DevicesMap,
		shmem_prefix: &str,
		images: &mut NameImagesMap,
		ipc_timeout: Duration,
	) -> Result<(), Box<dyn std::error::Error>> {
		// Get or create correct device
		let vk_device_entry =
			Self::get_or_create_device(vk_devices, vk_instance, cmd.gpu_device_uuid)?;
		let vk_device = vk_device_entry.get();

		let img_name_str = ImgData::convert_shmem_array_to_str(&cmd.image_name);
		let shmem_name_str = shmem_prefix.to_owned() + &img_name_str;

		let gpu_images_map = images.entry(img_name_str.clone()).or_default();

		// Find image data
		let img_loaded = gpu_images_map.images.contains_key(&cmd.gpu_device_uuid);

		// Process initialization
		let (result_msg_data, vk_shared_image, _lock) = if !img_loaded || cmd.overwrite_existing {
			// Only initialize image if none exists or the cmd explicitly allows overriding an image
			if !img_loaded {
				// Create image if it doesn't exist yet
				let ipc_info = IpcShmem::new(&shmem_name_str, &img_name_str, true)?;
				let vk_shared_image = VkCpuSharedImage::new(
					&vk_instance,
					&vk_device,
					1,
					1,
					vk::Format::R8G8B8A8_UNORM,
					0,
				)?;
				let _ = gpu_images_map
					.images
					.entry(cmd.gpu_device_uuid)
					.insert_entry(ServerImageData {
						ipc_info,
						vk_shared_image,
					});
			};

			// Acquire write lock to image
			// let lock = server_image_data
			// 	.acquire_lock(Timeout::Val(ipc_timeout))
			// 	.unwrap();
			// let mut data = IpcShmem::acquire_data(&lock);

			let format = VkSharedImage::get_vk_format(cmd.format);
			let mut cur_img_lock = MaybeUninit::uninit();
			let mut cur_img_data = MaybeUninit::uninit();
			let _locks = gpu_images_map
				.images
				.iter_mut()
				.map(|image| {
					// Update all shared images with the new size
					let lock = image.1.ipc_info.acquire_lock(Timeout::Val(ipc_timeout))?;
					let data = IpcShmem::acquire_data(&lock);

					image.1.vk_shared_image.borrow_mut().resize_image(
						&vk_instance,
						&vk_device,
						cmd.width,
						cmd.height,
						format,
						data.handle_id + 1,
						&mut gpu_images_map.ram_buffer,
					)?;

					// Update Shmem data
					VkServer::update_shmem_data(data, &image.1.vk_shared_image.image);

					if *image.0 == cmd.gpu_device_uuid {
						cur_img_lock = MaybeUninit::new(lock);
						cur_img_data = MaybeUninit::new(&image.1.vk_shared_image);
						Ok::<_, Box<dyn std::error::Error>>(None)
					} else {
						Ok::<_, Box<dyn std::error::Error>>(Some(lock))
					}
				})
				.collect::<Result<Vec<_>, _>>()?;

			let data = IpcShmem::acquire_data(unsafe { cur_img_lock.assume_init_ref() });

			// Generate ResultMsg data
			let img_data = ImgData::from_shmem_data_internal(
				ImgData::convert_shmem_str_to_array(&shmem_name_str),
				data.clone(),
			);

			// Return result, vk_shared_img, and lock
			(
				ResultInitImage {
					image_created: true,
					img_data,
				},
				Some(unsafe { cur_img_data.assume_init() }),
				Some(unsafe { cur_img_lock.assume_init() }),
			)
		} else {
			// If image not loaded or cmd.overwrite_existing is false, send empty result back
			(
				ResultInitImage {
					image_created: false,
					img_data: ImgData::default(),
				},
				None,
				None,
			)
		};

		// Send result to client
		let res_msg = ResultMsg {
			tag: CommandTag::InitImage,
			data: ResultData {
				init_img: ManuallyDrop::new(result_msg_data),
			},
		};
		connection.send_result(res_msg)?;

		// If image was created/updated, send handles to client
		if vk_shared_image.is_some() {
			let handles = vk_shared_image.unwrap().image.export_handle(vk_device)?;
			connection.send_anillary_handles(&[handles.into_raw_fd()])?;

			// Receive ack
			connection.recv_ack()?;
		}

		Ok(())
	}

	fn process_cmd_find_image(
		connection: &IpcConnection,
		cmd: &CommFindImage,
		vk_instance: &VkInstance,
		vk_devices: &mut DevicesMap,
		images: &mut NameImagesMap,
		ipc_timeout: Duration,
	) -> Result<(), Box<dyn std::error::Error>> {
		// Get or create correct device
		let vk_device_entry =
			Self::get_or_create_device(vk_devices, vk_instance, cmd.gpu_device_uuid)?;

		let vk_device = vk_device_entry.get();

		let img_name_str = ImgData::convert_shmem_array_to_str(&cmd.image_name);

		let gpu_images_map = images.entry(img_name_str).or_default();
		let image_and_lock: Option<(ImgData, &mut VkCpuSharedImage, ReadLockGuard)> =
			match gpu_images_map.images.entry(cmd.gpu_device_uuid) {
				Entry::Occupied(e) => {
					let entry = e.into_mut();
					let rlock = entry
						.ipc_info
						.acquire_rlock(Timeout::Val(ipc_timeout))
						.unwrap();
					let rdata = IpcShmem::acquire_rdata(&rlock);

					Some((
						ImgData::from_shmem_data_internal(
							ImgData::convert_shmem_str_to_array(entry.ipc_info.get_name()),
							rdata.clone(),
						),
						&mut entry.vk_shared_image,
						rlock,
					))
				}
				Entry::Vacant(_) => None,
			};

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
			let fd = vk_shared_image.unwrap().image.export_handle(vk_device)?;
			connection.send_anillary_handles(&[fd.into_raw_fd()])?;
			connection.recv_ack()?;
		}

		Ok(())
	}

	fn process_cmd_copy_image(
		connection: &IpcConnection,
		cmd: &CommCopyImage,
		vk_instance: &VkInstance,
		vk_devices: &mut DevicesMap,
		images: &mut NameImagesMap,
		ipc_timeout: Duration,
	) -> Result<(), Box<dyn std::error::Error>> {
		let img_name_str = ImgData::convert_shmem_array_to_str(&cmd.image_name);

		// Get gpu map
		let gpu_images_map = images.get(&img_name_str);
		if let Some(gpu_images_map) = gpu_images_map {
			// If only there's only one image in the map, no copy is necessary
			if gpu_images_map.images.len() <= 1 {
				return Ok(());
			}

			// Get read and write images
			let mut read_image = None;
			let mut read_lock = None;
			let mut write_images = Vec::new();
			write_images.reserve(gpu_images_map.images.len() - 1);
			let _write_locks = gpu_images_map
				.images
				.iter()
				.map(|image| {
					if *image.0 == cmd.gpu_device_uuid {
						read_lock =
							Some(image.1.ipc_info.acquire_rlock(Timeout::Val(ipc_timeout))?);
						read_image =
							Some((vk_devices.get(image.0).unwrap(), &image.1.vk_shared_image));
						Ok::<_, Box<dyn std::error::Error>>(None)
					} else {
						let write_lock =
							image.1.ipc_info.acquire_lock(Timeout::Val(ipc_timeout))?;
						write_images
							.push((vk_devices.get(image.0).unwrap(), &image.1.vk_shared_image));
						Ok::<_, Box<dyn std::error::Error>>(Some(write_lock))
					}
				})
				.collect::<Result<Vec<_>, _>>();

			if let Some(read_image) = read_image {
				VkCopyImages::copy_images(read_image, &write_images)?;
			}
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

	// fn update_shared_image<'a>(
	// 	cmd: &CommInitImage,
	// 	vk_instance: &VkInstance,
	// 	vk_device: &VkDevice,
	// 	image_vec: &'a mut NameImagesMap,
	// 	image_name: &str,
	// 	shmem_name: &str,
	// 	image_index: Option<usize>,
	// 	ipc_timeout: Duration,
	// ) -> Result<
	// 	(
	// 		ResultInitImage,
	// 		Option<&'a mut VkSharedImage>,
	// 		Option<LockGuard<'a>>,
	// 	),
	// 	Box<dyn std::error::Error>,
	// > {
	// 	// Check if an image with the given name is available
	// 	let image: &mut ServerImageData = {
	// 		if image_index.is_some() {
	// 			// Only overwrite image if explicitly requested
	// 			if !cmd.overwrite_existing {
	// 				return Ok((
	// 					ResultInitImage {
	// 						image_created: false,
	// 						img_data: ImgData::default(),
	// 					},
	// 					None,
	// 					None,
	// 				));
	// 			}

	// 			image_vec.get_mut(image_index.unwrap()).unwrap()
	// 		} else {
	// 			let ipc_info = IpcShmem::new(shmem_name, image_name, true)?;
	// 			let vk_shared_image = VkSharedImage::new(
	// 				&vk_instance,
	// 				&vk_device,
	// 				1,
	// 				1,
	// 				vk::Format::R8G8B8A8_UNORM,
	// 				0,
	// 			)?;
	// 			image_vec.push(Box::new(ServerImageData {
	// 				ipc_info,
	// 				vk_shared_image,
	// 			}));
	// 			image_vec.last_mut().unwrap()
	// 		}
	// 	};

	// 	// Update VkShared image and Shmem data
	// 	// Lock access
	// 	let lock = image
	// 		.ipc_info
	// 		.acquire_lock(Timeout::Val(ipc_timeout))
	// 		.unwrap();
	// 	let mut data = IpcShmem::acquire_data(&lock);

	// 	// Update VkSharedImage
	// 	image.vk_shared_image.resize_image(
	// 		&vk_instance,
	// 		&vk_device,
	// 		cmd.width,
	// 		cmd.height,
	// 		VkSharedImage::get_vk_format(cmd.format),
	// 		data.handle_id + 1,
	// 	)?;

	// 	// Update Shmem data
	// 	VkServer::update_shmem_data(&mut data, &image.vk_shared_image);

	// 	// Generate ResultMsg data
	// 	let img_data = ImgData::from_shmem_data_internal(
	// 		ImgData::convert_shmem_str_to_array(image.ipc_info.get_name()),
	// 		data.clone(),
	// 	);

	// 	// Return result, vk_shared_img, and lock
	// 	return Ok((
	// 		ResultInitImage {
	// 			image_created: true,
	// 			img_data,
	// 		},
	// 		Some(&mut image.vk_shared_image),
	// 		Some(lock),
	// 	));
	// }

	fn update_shmem_data(shmem_data: &mut ShmemDataInternal, vk_shared_image: &VkSharedImage) {
		let vk_data = vk_shared_image.get_image_data();

		shmem_data.width = vk_data.width;
		shmem_data.height = vk_data.height;
		shmem_data.format = VkSharedImage::get_img_format(vk_data.format);
		shmem_data.allocation_size = vk_data.allocation_size;
		shmem_data.handle_id = vk_data.id;
	}

	fn get_or_create_device<'a>(
		vk_devices: &'a mut DevicesMap,
		vk_instance: &VkInstance,
		gpu_device_uuid: u128,
	) -> Result<OccupiedEntry<'a, u128, VkDevice>, Box<dyn std::error::Error>> {
		// Check that a device with the given uuid is initialized
		let vk_device = match vk_devices.entry(gpu_device_uuid) {
			Entry::Occupied(o) => o,
			Entry::Vacant(v) => {
				let new_vk_device = VkDevice::new(
					&vk_instance,
					Some(VkPhysicalDeviceOptions {
						device_uuid: Some(uuid::Uuid::from_u128(gpu_device_uuid)),
						..Default::default()
					}),
				)
				.map_err(|err| err)?; // TODO: Handle wrong UUID
				v.insert_entry(new_vk_device)
			}
		};
		Ok(vk_device)
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
