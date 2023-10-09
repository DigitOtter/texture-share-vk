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
    daemon_timeout_in_millis: u128,
    spawn_timeout: Duration,
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
            daemon_timeout_in_millis,
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
    daemon_timeout_in_millis: u128,
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
            daemon_timeout_in_millis,
        )?);
    }

    return Ok(None);
}

fn spawn(
    program_path: &str,
    lock_file_path: &str,
    socket_path: &str,
    shmem_prefix: &str,
    daemon_timeout_in_millis: u128,
) -> Result<process::Child, Error> {
    process::Command::new(program_path)
        .args([
            format!("--lock-file={}", lock_file_path),
            format!("--socket-lock-file={}", socket_path),
            format!("--shmem-prefix={}", shmem_prefix),
            format!("--timeout-millis={}", daemon_timeout_in_millis),
        ])
        .spawn()
}
