use std::{
	fs::{self, OpenOptions},
	path::Path,
	sync::{atomic::AtomicBool, Arc},
	time::{Duration, SystemTime},
};

use clap::Parser;
use fs2::FileExt;
use texture_share_vk_server::VkServer;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
	#[arg(short, long, default_value = "test.lock")]
	lock_file: String,

	#[arg(short, long, default_value = "test.sock")]
	socket_file: String,

	#[arg(long, default_value = "shared_image_")]
	shmem_prefix: String,

	#[arg(long, default_value_t = 2000)]
	socket_timeout_millis: u64,

	#[arg(long, default_value_t = 2000)]
	connection_wait_timeout_millis: u64,

	#[arg(long, default_value_t = 2000)]
	ipc_timeout_millis: u64,

	#[arg(long, default_value_t = 2000)]
	lockfile_timeout_millis: u64,

	#[arg(long, required = false)]
	gpu_vendor_id: Option<u32>,

	#[arg(long, required = false)]
	gpu_device_id: Option<u32>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
	let args = Args::parse();

	let lock_file_path = Path::new(&args.lock_file);
	fs::create_dir_all(&lock_file_path.parent().unwrap_or(Path::new(".")))?;

	// Take ownership of lock_file
	let lock_file = {
		let stop_time = SystemTime::now() + Duration::from_millis(args.lockfile_timeout_millis);
		loop {
			let file = OpenOptions::new()
				.create(true)
				.write(true)
				.open(lock_file_path)?;
			let lock_res = file.try_lock_exclusive();

			if lock_res.is_ok() {
				break Ok(file);
			}

			if SystemTime::now() > stop_time {
				break Err(lock_res.err().unwrap());
			}
		}
	}
	.expect(&format!(
		"Failed to acquire lock for file {:?}",
		args.lockfile_timeout_millis
	));

	let _ = fs::remove_file(&args.socket_file);

	// Check if GPU vendor and device ID's were submitted
	let gpu_vendor_device_ids = if args.gpu_vendor_id.is_some() && args.gpu_device_id.is_some() {
		Some((args.gpu_vendor_id.unwrap(), args.gpu_device_id.unwrap()))
	} else {
		None
	};

	let vk_server = VkServer::new(
		&args.socket_file,
		&args.shmem_prefix,
		Duration::from_millis(args.socket_timeout_millis),
		Duration::from_millis(args.connection_wait_timeout_millis),
		Duration::from_millis(args.ipc_timeout_millis),
		gpu_vendor_device_ids,
	)?;

	vk_server.loop_server(Arc::new(AtomicBool::new(false)))?;

	// File cleanup
	let _ = fs::remove_file(&args.socket_file);
	let _ = fs::remove_file(lock_file_path);
	lock_file.unlock()?;

	Ok(())
}
