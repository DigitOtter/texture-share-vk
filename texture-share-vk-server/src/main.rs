#![feature(cstr_count_bytes)]

use std::{
	ffi::CString,
	fs::{self, OpenOptions},
	path::Path,
	str::FromStr,
	sync::{atomic::AtomicBool, Arc},
	time::{Duration, SystemTime},
};

use clap::{builder::TypedValueParser, Parser};
use fs2::FileExt;
use texture_share_vk_base::{uuid, vk_device::VkPhysicalDeviceOptions};
use texture_share_vk_server::VkServer;

#[derive(Clone)]
struct UuidParser;

impl TypedValueParser for UuidParser {
	type Value = uuid::Uuid;

	fn parse_ref(
		&self,
		cmd: &clap::Command,
		arg: Option<&clap::Arg>,
		value: &std::ffi::OsStr,
	) -> Result<Self::Value, clap::Error> {
		let inner = clap::builder::StringValueParser::new();
		let val = inner.parse_ref(cmd, arg, value)?;

		let uuid = uuid::Uuid::from_str(val.as_str()).map_err(|e| {
			let mut err = clap::Error::new(clap::error::ErrorKind::ValueValidation).with_cmd(cmd);
			if let Some(arg) = arg {
				err.insert(
					clap::error::ContextKind::InvalidArg,
					clap::error::ContextValue::String(arg.to_string()),
				);
			}
			err.insert(
				clap::error::ContextKind::InvalidValue,
				clap::error::ContextValue::String(format!("{}", e.to_string())),
			);

			err
		})?;

		Ok(uuid)
	}
}

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

	#[arg(long, required = false)]
	gpu_device_name: Option<String>,

	#[arg(long, required = false, value_parser=clap::builder::ValueParser::new(UuidParser{}))]
	gpu_device_uuid: Option<uuid::Uuid>,
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
	let physical_device_properties = VkPhysicalDeviceOptions {
		vendor_id: args.gpu_vendor_id,
		device_id: args.gpu_device_id,
		device_uuid: args.gpu_device_uuid,
		device_name: args
			.gpu_device_name
			.map(|x| CString::new(x).expect("Failed to get GPU device name")),
		..Default::default()
	};

	let vk_server = VkServer::new(
		&args.socket_file,
		&args.shmem_prefix,
		Duration::from_millis(args.socket_timeout_millis),
		Duration::from_millis(args.connection_wait_timeout_millis),
		Duration::from_millis(args.ipc_timeout_millis),
		Some(physical_device_properties),
	)?;

	vk_server.loop_server(Arc::new(AtomicBool::new(false)))?;

	// File cleanup
	let _ = fs::remove_file(&args.socket_file);
	let _ = fs::remove_file(lock_file_path);
	lock_file.unlock()?;

	Ok(())
}
