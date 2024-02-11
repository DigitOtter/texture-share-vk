use std::{
	io::Error,
	process::{self, Child},
	time::{Duration, SystemTime},
};

// Tries to connect to server. If that fails, spawn daemon and retry
pub fn server_connect_and_daemon_launch<T>(
	program_path: &str,
	lock_file_path: &str,
	socket_path: &str,
	shmem_prefix: &str,
	socket_timeout: Duration,
	connection_wait_timeout: Duration,
	ipc_timeout: Duration,
	lockfile_timeout: Duration,
	spawn_timeout: Duration,
	gpu_device_uuid: Option<uuid::Uuid>,
	f: &dyn Fn() -> Result<Option<T>, Error>,
) -> Result<Option<T>, Error> {
	let stop_time = SystemTime::now() + spawn_timeout;
	let mut child = Box::new(None);
	let conn: Result<Option<T>, Error> = loop {
		// Execute function to launch and connect client
		let conn = try_connect(
			&mut child,
			program_path,
			lock_file_path,
			socket_path,
			shmem_prefix,
			socket_timeout,
			connection_wait_timeout,
			ipc_timeout,
			lockfile_timeout,
			gpu_device_uuid,
			f,
		)?;

		// Return client if a connection was established
		if conn.is_some() {
			break Ok(conn);
		}

		if SystemTime::now() > stop_time {
			// Kill child if no connection could be established in time
			break Ok(None);
		}
	};

	if conn.is_ok() {
		// Return connection. Note that child process is not killed on Drop
		return conn;
	} else {
		// If an error occured or the connection could not be established in time, kill child
		if child.is_some() {
			child.unwrap().kill()?;
		}
		return Ok(None);
	}
}

fn try_connect<T>(
	child: &mut Box<Option<Child>>,
	program_path: &str,
	lock_file_path: &str,
	socket_path: &str,
	shmem_prefix: &str,
	socket_timeout: Duration,
	connection_wait_timeout: Duration,
	ipc_timeout: Duration,
	lockfile_timeout: Duration,
	gpu_device_uuid: Option<uuid::Uuid>,
	f: &dyn Fn() -> Result<Option<T>, Error>,
) -> Result<Option<T>, Error> {
	// Execute function to launch and connect client
	let res = f()?;
	if res.is_some() {
		return Ok(res);
	}

	// (Re-)start child if process has not been started or has exited
	if child.is_none() || child.as_mut().as_mut().unwrap().try_wait()?.is_some() {
		// Kill running process
		if child.is_some() {
			child
				.as_mut()
				.as_mut()
				.expect("Failed to get child process handle")
				.kill()?;
		}

		// Spawn new process
		*child.as_mut() = Some(spawn(
			program_path,
			lock_file_path,
			socket_path,
			shmem_prefix,
			socket_timeout.as_millis(),
			connection_wait_timeout.as_millis(),
			ipc_timeout.as_millis(),
			lockfile_timeout.as_millis(),
			gpu_device_uuid,
		)?);
	}

	return Ok(None);
}

fn spawn(
	program_path: &str,
	lock_file_path: &str,
	socket_path: &str,
	shmem_prefix: &str,
	socket_timeout_in_millis: u128,
	connection_wait_timeout_in_millis: u128,
	ipc_timeout_in_millis: u128,
	lockfile_timeout_in_millis: u128,
	gpu_device_uuid: Option<uuid::Uuid>,
) -> Result<process::Child, Error> {
	let mut args = vec![
		format!("--lock-file={}", lock_file_path),
		format!("--socket-file={}", socket_path),
		format!("--shmem-prefix={}", shmem_prefix),
		format!("--socket-timeout-millis={}", socket_timeout_in_millis),
		format!(
			"--connection-wait-timeout-millis={}",
			connection_wait_timeout_in_millis
		),
		format!("--ipc-timeout-millis={}", ipc_timeout_in_millis),
		format!("--lockfile-timeout-millis={}", lockfile_timeout_in_millis),
	];
	if gpu_device_uuid.is_some() {
		args.push(format!(
			"--gpu-device-uuid={}",
			gpu_device_uuid.unwrap().to_string()
		));
	}

	process::Command::new(program_path).args(args).spawn()
}
