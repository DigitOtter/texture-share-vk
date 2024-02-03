use std::{
	ffi::CStr,
	fs,
	sync::{
		atomic::{AtomicBool, Ordering},
		Arc,
	},
	thread,
	time::Duration,
};

use texture_share_vk_base::{ash::vk, vk_device::VkDevice, vk_instance::VkInstance};
use texture_share_vk_base::{
	ipc::platform::img_data::ImgFormat, vk_setup::VkSetup, vk_shared_image::VkSharedImage,
};
use texture_share_vk_client::VkClient;
use texture_share_vk_server::VkServer;

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

fn _client_create() -> VkClient {
	let vk_instance =
		VkInstance::new(None, CStr::from_bytes_with_nul(b"VkClient\0").unwrap()).unwrap();
	let vk_device = VkDevice::new(&vk_instance, None).unwrap();
	let vk_setup = Box::new(VkSetup::new(vk_instance, vk_device));

	VkClient::new(SOCKET_PATH, vk_setup, SOCKET_TIMEOUT)
		.expect("Client failed to connect to server")
}

#[test]
fn server_client_connect() {
	let _ = fs::remove_file(SOCKET_PATH);

	let stop_bit = Arc::new(AtomicBool::new(false));

	let stop_clone = stop_bit.clone();
	let server_fcn = move || {
		let server = _server_create();
		server.loop_server(stop_clone).expect("Server loop failed")
	};

	let client_fcn = move || {
		let _ = _client_create();
		println!("Connection successful");
	};

	let server_thread = thread::spawn(server_fcn);
	let client_thread = thread::spawn(client_fcn);

	thread::sleep(Duration::from_secs(2));
	loop {
		stop_bit.clone().store(true, Ordering::Relaxed);

		if server_thread.is_finished() && client_thread.is_finished() {
			break;
		}
	}

	client_thread.join().unwrap();
	server_thread.join().unwrap();
}

#[test]
fn server_client_init_image() {
	let _ = fs::remove_file(SOCKET_PATH);

	const IMAGE_NAME: &str = "test_img";

	let stop_bit = Arc::new(AtomicBool::new(false));

	let stop_clone = stop_bit.clone();
	let server_fcn = move || {
		let server = _server_create();
		server.loop_server(stop_clone).expect("Server loop failed")
	};

	let client_fcn = move || {
		let mut client = _client_create();
		println!("Connection successful");

		let res = client
			.init_image(IMAGE_NAME, 1, 1, ImgFormat::R8G8B8A8, false)
			.unwrap();

		assert!(res.is_some());
		println!("Image created");
	};

	let server_thread = thread::spawn(server_fcn);
	let client_thread = thread::spawn(client_fcn);

	thread::sleep(Duration::from_secs(2));
	loop {
		stop_bit.clone().store(true, Ordering::Relaxed);

		if server_thread.is_finished() && client_thread.is_finished() {
			break;
		}
	}

	client_thread.join().unwrap();
	server_thread.join().unwrap();
}

#[test]
fn server_client_overwrite_image() {
	let _ = fs::remove_file(SOCKET_PATH);

	const IMAGE_NAME: &str = "test_img";

	let stop_bit = Arc::new(AtomicBool::new(false));

	let stop_clone = stop_bit.clone();
	let server_fcn = move || {
		let server = _server_create();
		server.loop_server(stop_clone).expect("Server loop failed")
	};

	let client_fcn = move || {
		let mut client = _client_create();
		println!("Connection successful");

		let res = client
			.init_image(IMAGE_NAME, 1, 1, ImgFormat::R8G8B8A8, false)
			.unwrap();

		assert!(res.is_some());
		println!("Image created");

		let res = client
			.init_image(IMAGE_NAME, 1, 1, ImgFormat::R8G8B8A8, false)
			.unwrap();

		assert!(res.is_none());
		println!("Image not overwritten, as expected");

		let res = client
			.init_image(IMAGE_NAME, 2, 2, ImgFormat::R8G8B8A8, true)
			.unwrap();

		assert!(res.is_some());
		println!("Image overwritten");
	};

	let server_thread = thread::spawn(server_fcn);
	let client_thread = thread::spawn(client_fcn);

	thread::sleep(Duration::from_secs(2));
	loop {
		stop_bit.clone().store(true, Ordering::Relaxed);

		if server_thread.is_finished() && client_thread.is_finished() {
			break;
		}
	}

	client_thread.join().unwrap();
	server_thread.join().unwrap();
}

#[test]
fn server_client_find_image() {
	let _ = fs::remove_file(SOCKET_PATH);

	const IMAGE_NAME: &str = "test_img";

	let stop_bit = Arc::new(AtomicBool::new(false));

	let stop_clone = stop_bit.clone();
	let server_fcn = move || {
		let server = _server_create();
		server.loop_server(stop_clone).expect("Server loop failed")
	};

	let client_fcn = move || {
		let mut client = _client_create();
		println!("Connection successful");

		let res = client.find_image(IMAGE_NAME, false).unwrap();
		assert!(res.is_none());
		println!("Image not found, as expected");

		let res = client.find_image(IMAGE_NAME, true).unwrap();
		assert!(res.is_none());
		println!("Image not found, as expected");

		let res = client
			.init_image(IMAGE_NAME, 1, 1, ImgFormat::R8G8B8A8, false)
			.unwrap();

		assert!(res.is_some());
		println!("Image created");

		let res = client.find_image(IMAGE_NAME, false).unwrap();
		assert!(res.is_some());
		println!("Image found from import");

		let res = client.find_image(IMAGE_NAME, false).unwrap();
		assert!(res.is_some());
		println!("Image found in local cache");

		let res = client.find_image(IMAGE_NAME, true).unwrap();
		assert!(res.is_some());
		println!("Image found from forced import");
	};

	let server_thread = thread::spawn(server_fcn);
	let client_thread = thread::spawn(client_fcn);

	thread::sleep(Duration::from_secs(2));
	loop {
		stop_bit.clone().store(true, Ordering::Relaxed);

		if server_thread.is_finished() && client_thread.is_finished() {
			break;
		}
	}

	client_thread.join().unwrap();
	server_thread.join().unwrap();
}

#[test]
fn server_client_find_image_data() {
	let _ = fs::remove_file(SOCKET_PATH);

	const IMAGE_NAME: &str = "test_img";

	let stop_bit = Arc::new(AtomicBool::new(false));

	let stop_clone = stop_bit.clone();
	let server_fcn = move || {
		let server = _server_create();
		server.loop_server(stop_clone).expect("Server loop failed")
	};

	let client_fcn = move || {
		let mut client = _client_create();
		println!("Connection successful");

		{
			let res = client.find_image_data(IMAGE_NAME, false).unwrap();
			assert!(res.is_none());
			println!("Image data not found, as expected");
		}

		let width: u32 = 1;
		let height: u32 = 2;
		let format: ImgFormat = ImgFormat::R8G8B8A8;
		{
			let res = client
				.init_image(IMAGE_NAME, width, height, format, false)
				.unwrap();
			assert!(res.is_some());
			println!("Image created");
		}

		let id = {
			let res = client.find_image_data(IMAGE_NAME, false).unwrap();
			assert!(res.is_some());
			println!("Image found from import");

			let (_lock, data) = res.unwrap();
			assert_eq!(data.format, format);
			assert_eq!(data.width, width);
			assert_eq!(data.height, height);

			data.handle_id
		};

		{
			let res = client
				.init_image(IMAGE_NAME, width + 1, height + 1, format, true)
				.unwrap();
			assert!(res.is_some());
			println!("Image overwritten");

			let res = client.find_image_data(IMAGE_NAME, false).unwrap();
			assert!(res.is_some());
			println!("Image found from import");

			assert_ne!(
				id,
				res.unwrap().1.handle_id,
				"Handle was not updated between rewrite"
			);
		}
	};

	let server_thread = thread::spawn(server_fcn);
	let client_thread = thread::spawn(client_fcn);

	thread::sleep(Duration::from_secs(2));
	loop {
		stop_bit.clone().store(true, Ordering::Relaxed);

		if server_thread.is_finished() && client_thread.is_finished() {
			break;
		}
	}

	client_thread.join().unwrap();
	server_thread.join().unwrap();
}

#[test]
fn server_client_send_image() {
	let _ = fs::remove_file(SOCKET_PATH);

	const IMAGE_NAME: &str = "test_img";

	let stop_bit = Arc::new(AtomicBool::new(false));

	let stop_clone = stop_bit.clone();
	let server_fcn = move || {
		let server = _server_create();
		server.loop_server(stop_clone).expect("Server loop failed")
	};

	let client_fcn = move || {
		let mut client = _client_create();
		println!("Connection successful");

		let res = client
			.init_image(IMAGE_NAME, 1, 1, ImgFormat::R8G8B8A8, false)
			.unwrap();
		assert!(res.is_some());
		println!("Image created");

		let local_image = VkSharedImage::new(
			&client.get_vk_setup().instance,
			&client.get_vk_setup().device,
			1,
			1,
			vk::Format::R8G8B8A8_UNORM,
			0,
		)
		.unwrap();

		let fence = client.get_vk_setup().device.create_fence(None).unwrap();
		let res = client
			.send_image(
				IMAGE_NAME,
				local_image.image,
				local_image.image_layout,
				local_image.image_layout,
				fence,
			)
			.unwrap();

		assert!(res.is_some(), "Failed to send image");
		println!("Image sent");

		let res = client
			.recv_image(
				IMAGE_NAME,
				local_image.image,
				local_image.image_layout,
				local_image.image_layout,
				fence,
			)
			.unwrap();

		assert!(res.is_some(), "Failed to receive image");
		println!("Image received");

		client.get_vk_setup().device.destroy_fence(fence);
		local_image.destroy(&client.get_vk_setup().device);
	};

	let server_thread = thread::spawn(server_fcn);
	let client_thread = thread::spawn(client_fcn);

	thread::sleep(Duration::from_secs(2));
	loop {
		stop_bit.clone().store(true, Ordering::Relaxed);

		if server_thread.is_finished() && client_thread.is_finished() {
			break;
		}
	}

	client_thread.join().unwrap();
	server_thread.join().unwrap();
}
