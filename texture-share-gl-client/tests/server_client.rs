use std::{
	fs,
	sync::{
		atomic::{AtomicBool, Ordering},
		Arc,
	},
	thread,
	time::Duration,
};

// use texture_share_vk_base::ipc::platform::img_data::ImgFormat;
// use texture_share_vk_base::{
//     vk_setup::ffi::vk_setup_new,
//     vk_shared_image::ffi::{vk_shared_image_new, VkFormat},
// };
use texture_share_gl_client::{
	gl_shared_image::{
		ffi::{gl_external_initialize, gl_shared_image_new, GlFormat, SharedImageData},
		GLsizei, GLuint, GLuint64,
	},
	GlClient,
};
use texture_share_ipc::platform::img_data::ImgFormat;
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
	)
	.unwrap()
}

fn _client_create() -> GlClient {
	gl_external_initialize();
	GlClient::new(SOCKET_PATH, SOCKET_TIMEOUT).expect("Client failed to connect to server")
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

	thread::sleep(Duration::from_secs(1));
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

	thread::sleep(Duration::from_secs(1));
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

	thread::sleep(Duration::from_secs(1));
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

	thread::sleep(Duration::from_secs(1));
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

	thread::sleep(Duration::from_secs(1));
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

		let width = 1;
		let height = 1;
		let allocation_size = width * height * 4;
		let mut local_image = gl_shared_image_new();
		local_image.as_mut().unwrap().initialize(
			GLsizei(width),
			GLsizei(height.into()),
			0,
			GLuint64(allocation_size as u64),
			GlFormat::RGBA,
			SharedImageData::GL_RGBA8,
		);

		let res = client
			.send_image(
				IMAGE_NAME,
				local_image.get_texture_id(),
				SharedImageData::GL_TEXTURE_2D,
				false,
				GLuint(0),
			)
			.unwrap();

		assert!(res.is_some(), "Failed to send image");
		println!("Image sent");

		let res = client
			.recv_image(
				IMAGE_NAME,
				local_image.get_texture_id(),
				SharedImageData::GL_TEXTURE_2D,
				false,
				GLuint(0),
			)
			.unwrap();

		assert!(res.is_some(), "Failed to receive image");
		println!("Image received");
	};

	let server_thread = thread::spawn(server_fcn);
	let client_thread = thread::spawn(client_fcn);

	thread::sleep(Duration::from_secs(1));
	loop {
		stop_bit.clone().store(true, Ordering::Relaxed);

		if server_thread.is_finished() && client_thread.is_finished() {
			break;
		}
	}

	client_thread.join().unwrap();
	server_thread.join().unwrap();
}
