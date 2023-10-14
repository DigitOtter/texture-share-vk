use std::cell::RefCell;
use std::cmp::min;
use std::io::{Error, ErrorKind, IoSlice, IoSliceMut, Read, Write};
use std::mem::size_of;
use std::os::fd::{FromRawFd, OwnedFd, RawFd};
use std::os::unix::net::{AncillaryData, SocketAncillary, UnixListener, UnixStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime};

use crate::platform::ipc_commands::{CommandMsg, ResultMsg};

pub struct IpcConnection {
	conn: RefCell<UnixStream>,
	//proc_id: i32,
	timeout: Duration,
}

pub struct IpcSocket {
	listener_socket: UnixListener,
	pub connections: Arc<Mutex<Vec<RefCell<IpcConnection>>>>,
	pub timeout: Duration,
}

impl IpcConnection {
	pub fn new(conn: UnixStream, timeout: Duration) -> IpcConnection {
		conn.set_nonblocking(true).unwrap();

		// TODO: Use socket timeout instead of own implementation
		// conn.set_read_timeout(Some(timeout)).unwrap();
		// conn.set_write_timeout(Some(timeout)).unwrap();

		IpcConnection {
			conn: RefCell::new(conn),
			//proc_id,
			timeout,
		}
	}

	pub fn get_socket(&self) -> std::cell::Ref<'_, UnixStream> {
		return self.conn.borrow();
	}

	pub fn try_connect(
		socket_path: &str,
		timeout: Duration,
	) -> Result<Option<IpcConnection>, Error> {
		IpcConnection::try_fcn_timeout(
			|| {
				//let sock = UnixStream::unbound()?;
				match UnixStream::connect(socket_path) {
					Ok(c) => Ok(Some(IpcConnection::new(c, timeout))),
					Err(e) => match e.kind() {
						ErrorKind::AddrNotAvailable
						| ErrorKind::NotFound
						| ErrorKind::Interrupted => Ok(None),
						_ => Err(e),
					},
				}
			},
			&timeout,
			&IpcConnection::sleep_interval(timeout),
		)
	}

	fn compute_cmsg_header_size() -> usize {
		#[cfg(target_pointer_width = "64")]
		return 8 + 3 * 4;
		#[cfg(target_pointer_width = "32")]
		return 4 + 3 * 4;
	}

	pub fn send_anillary_handles(&self, handles: &[RawFd]) -> Result<usize, Error> {
		if handles.len() == 0 {
			Ok(0)
		} else {
			let buf = [0 as u8; 4];

			let abuf_len =
				IpcConnection::compute_cmsg_header_size() + handles.len() * size_of::<RawFd>();
			let mut abuf = vec![0 as u8; abuf_len];
			let mut ancillary = SocketAncillary::new(&mut abuf);
			if !ancillary.add_fds(handles) {
				// This means that the compute_cmsg_header_size() fcn is incorrect
				panic!("Failed to add file descriptors to ancillary data");
			}
			self.conn
				.borrow()
				.send_vectored_with_ancillary(&[IoSlice::new(&buf)], &mut ancillary)
			// let ancillary =
			//     [AncillaryData::FileDescriptors(std::borrow::Cow::Borrowed(handles)); 1];
			// self.conn
			//     .send_ancillary(&[1 as u8; 4], ancillary)
			//     .map(|r| r.1)
		}
	}

	pub fn recv_ancillary(&self, handle_count: usize) -> Result<Vec<OwnedFd>, Error> {
		let mut buf = [0 as u8; 4];
		let mut fds = Vec::<OwnedFd>::new();
		let _ = IpcConnection::try_fcn_timeout(
			|| {
				let abuf_len = IpcConnection::compute_cmsg_header_size()
					+ (handle_count - fds.len()) * size_of::<RawFd>();
				let mut abuf = vec![0 as u8; abuf_len];
				let mut adat = SocketAncillary::new(&mut abuf);
				let rec = self
					.conn
					.borrow()
					.recv_vectored_with_ancillary(&mut [IoSliceMut::new(&mut buf)], &mut adat)?;

				if rec > 0 {
					for ares in adat.messages() {
						if let AncillaryData::ScmRights(afds) = ares.unwrap() {
							for fd in afds {
								fds.push(unsafe { OwnedFd::from_raw_fd(fd) })
							}
						}
					}

					if fds.len() >= handle_count {
						Ok(Some(fds.len()))
					} else {
						Ok(None)
					}
				} else {
					Ok(None)
				}
			},
			&self.timeout,
			&IpcConnection::sleep_interval(self.timeout),
		)?;

		Ok(fds)
	}

	pub fn send_command(&self, command_msg: CommandMsg) -> Result<(), Error> {
		const MSG_LEN: usize = size_of::<CommandMsg>();
		let raw_ptr: *const CommandMsg = &(command_msg);
		let msg: &[u8; MSG_LEN] = unsafe { raw_ptr.cast::<[u8; MSG_LEN]>().as_ref().unwrap() };
		self.conn.borrow_mut().write_all(msg)
	}

	pub fn send_result(&self, result_msg: ResultMsg) -> Result<(), Error> {
		const MSG_LEN: usize = size_of::<ResultMsg>();
		let raw_ptr: *const ResultMsg = &(result_msg);
		let msg: &[u8; MSG_LEN] = unsafe { raw_ptr.cast::<[u8; MSG_LEN]>().as_ref().unwrap() };
		self.conn.borrow_mut().write_all(msg)
	}

	pub fn recv_command_if_available(&self) -> Result<Option<CommandMsg>, Error> {
		let mut msg = CommandMsg::default();

		const MSG_LEN: usize = size_of::<CommandMsg>();
		let buf: &mut [u8; MSG_LEN] = unsafe {
			(&mut msg as *mut CommandMsg)
				.cast::<[u8; MSG_LEN]>()
				.as_mut()
				.unwrap()
		};

		// Check if a message is waiting
		let first_read = match self.conn.borrow_mut().read(buf) {
			Err(e) => match e.kind() {
				ErrorKind::WouldBlock => Ok(0 as usize),
				_ => Err(e),
			},
			s => s,
		}?;
		if first_read == 0 {
			return Ok(None);
		}

		let mut rec_bytes: usize = first_read;
		let recv_res = IpcConnection::try_fcn_timeout(
			|| {
				let rec_buf: &mut [u8] = buf.split_at_mut(rec_bytes).1;
				self.conn.borrow_mut().read_exact(rec_buf)?;
				rec_bytes += rec_buf.len();

				if rec_bytes >= MSG_LEN {
					Ok::<Option<()>, Error>(Some(()))
				} else {
					Ok(None)
				}
			},
			&self.timeout,
			&IpcConnection::sleep_interval(self.timeout),
		)?;

		Ok(recv_res.and_then(|_| Some(msg)))
	}

	#[allow(dead_code)]
	pub fn recv_command(&self) -> Result<Option<CommandMsg>, Error> {
		let mut msg = CommandMsg::default();

		const MSG_LEN: usize = size_of::<CommandMsg>();
		let buf: &mut [u8; MSG_LEN] = unsafe {
			(&mut msg as *mut CommandMsg)
				.cast::<[u8; MSG_LEN]>()
				.as_mut()
				.unwrap()
		};

		let mut rec_bytes: usize = 0;
		let recv_res = IpcConnection::try_fcn_timeout(
			|| {
				let rec_buf: &mut [u8] = buf.split_at_mut(rec_bytes).1;
				self.conn.borrow_mut().read_exact(rec_buf)?;
				rec_bytes += rec_buf.len();

				if rec_bytes >= MSG_LEN {
					Ok::<Option<()>, Error>(Some(()))
				} else {
					Ok(None)
				}
			},
			&self.timeout,
			&IpcConnection::sleep_interval(self.timeout),
		)?;

		Ok(recv_res.and_then(|_| Some(msg)))
	}

	pub fn recv_result(&self) -> Result<Option<ResultMsg>, Error> {
		let mut msg = ResultMsg::default();

		const MSG_LEN: usize = size_of::<ResultMsg>();
		let buf: &mut [u8; MSG_LEN] = unsafe {
			(&mut msg as *mut ResultMsg)
				.cast::<[u8; MSG_LEN]>()
				.as_mut()
				.unwrap()
		};

		let mut rec_bytes: usize = 0;
		let recv_res = IpcConnection::try_fcn_timeout(
			|| {
				let rec_buf = buf.split_at_mut(rec_bytes).1;
				self.conn.borrow_mut().read_exact(rec_buf)?;
				rec_bytes += rec_buf.len();

				if rec_bytes >= MSG_LEN {
					Ok::<Option<()>, Error>(Some(()))
				} else {
					Ok(None)
				}
			},
			&self.timeout,
			&IpcConnection::sleep_interval(self.timeout),
		)?;

		Ok(recv_res.and_then(|_| Some(msg)))
	}

	pub fn send_ack(&self) -> Result<(), Error> {
		self.conn.borrow_mut().write_all(&[1 as u8]).map(|_| ())
	}

	pub fn recv_ack(&self) -> Result<Option<()>, Error> {
		let mut buf = [0 as u8];
		IpcConnection::try_fcn_timeout(
			|| {
				self.conn
					.borrow_mut()
					.read_exact(&mut buf)
					.map(|_| Some(()))
			},
			&self.timeout,
			&IpcConnection::sleep_interval(self.timeout),
		)
	}

	fn try_fcn_timeout<R, F: FnMut() -> Result<Option<R>, Error>>(
		mut f: F,
		timeout: &Duration,
		sleep_time: &Duration,
	) -> Result<Option<R>, Error> {
		let start_time = SystemTime::now();
		loop {
			let r = match f() {
				Err(e) => match e.kind() {
					ErrorKind::WouldBlock => None,
					_ => Err(e)?,
				},
				Ok(r) => r,
			};

			if r.is_some() {
				break Ok(r);
			}

			if SystemTime::now().duration_since(start_time).unwrap() > *timeout {
				break Ok(None);
			}

			thread::sleep(*sleep_time);
		}
	}

	fn sleep_interval(timeout: Duration) -> Duration {
		const MIN_SLEEP_DUR: Duration = Duration::from_millis(100);
		min(MIN_SLEEP_DUR, timeout / 10)
	}
}

impl IpcSocket {
	pub fn new(socket_path: &str, timeout: Duration) -> Result<IpcSocket, Error> {
		let listener_socket = IpcConnection::try_fcn_timeout(
			|| match UnixListener::bind(socket_path) {
				Err(e) => match e.kind() {
					ErrorKind::AddrInUse
					| ErrorKind::AddrNotAvailable
					| ErrorKind::AlreadyExists => Ok(None),
					_ => Err(e),
				},
				Ok(r) => Ok(Some(r)),
			},
			&timeout,
			&IpcConnection::sleep_interval(timeout),
		)?
		.expect("Failed to create socket");
		listener_socket.set_nonblocking(true)?;

		Ok(IpcSocket {
			listener_socket,
			connections: Arc::new(Mutex::new(Vec::new())),
			timeout,
		})
	}

	pub fn get_socket(&self) -> &UnixListener {
		return &self.listener_socket;
	}

	pub fn try_accept(&self) -> Result<Option<()>, Error> {
		let res = IpcConnection::try_fcn_timeout(
			|| {
				//print!("Trying to accept\n");
				match self.listener_socket.accept() {
					Err(e) => match e.kind() {
						ErrorKind::WouldBlock => Ok(None),
						_ => Err(e),
					},
					Ok(c) => {
						let ipc_conn = IpcConnection::new(c.0, self.timeout);
						self.connections
							.lock()
							.unwrap()
							.push(RefCell::new(ipc_conn));
						Ok(Some(()))
					}
				}
			},
			&self.timeout,
			&IpcConnection::sleep_interval(self.timeout),
		)?;

		if res.is_some() {
			Ok(Some(()))
		} else {
			Ok(None)
		}
	}
}

#[cfg(test)]
mod tests {
	use std::{fs, os::fd::AsRawFd};

	use super::*;

	const TIMEOUT: Duration = Duration::from_millis(10000);
	const SOCK_PATH: &str = "test_socket.sock";

	#[test]
	fn socket_creation() {
		let _ = fs::remove_file(SOCK_PATH);
		let _ = IpcSocket::new(SOCK_PATH, TIMEOUT).unwrap();
	}

	fn _ipc_stream_create() -> (IpcSocket, IpcConnection) {
		thread::sleep(Duration::from_secs(1));

		let listen_thread = move || {
			let listener = IpcSocket::new(SOCK_PATH, TIMEOUT)?;
			let _ = listener
				.try_accept()?
				.expect("Failed to listen for connection");
			Ok::<_, Error>(listener)
		};

		let connect_thread = || {
			let conn = IpcConnection::try_connect(SOCK_PATH, TIMEOUT)?
				.expect("Failed to connect to socket");
			Ok::<_, Error>(conn)
		};

		let listen_handle = thread::spawn(listen_thread);
		let connect_handle = thread::spawn(connect_thread);
		let listener = listen_handle.join().unwrap().unwrap();
		let conn = connect_handle.join().unwrap().unwrap();

		(listener, conn)
	}

	#[test]
	fn ipc_stream_create() {
		let _ = fs::remove_file(SOCK_PATH);
		let _ = _ipc_stream_create();
	}

	#[test]
	fn ipc_ack() {
		let _ = fs::remove_file(SOCK_PATH);
		let (listener, conn) = _ipc_stream_create();

		let conn_vector = listener.connections.clone();
		let send_thread = move || {
			conn_vector
				.lock()
				.unwrap()
				.last()
				.unwrap()
				.borrow_mut()
				.send_ack()
		};

		let recv_thread = move || conn.recv_ack();

		let s_handle = thread::spawn(send_thread);
		let r_handle = thread::spawn(recv_thread);

		let _ = s_handle.join().unwrap().expect("Failed to send ack");
		let _ = r_handle
			.join()
			.unwrap()
			.unwrap()
			.expect("Failed to receive ack");
	}

	#[test]
	fn ipc_msg() {
		let _ = fs::remove_file(SOCK_PATH);
		let (listener, conn) = _ipc_stream_create();

		let conn_vector = listener.connections.clone();
		let send_thread = move || {
			let mut msg = CommandMsg::default();
			unsafe { (*msg.data.find_img).image_name.fill(1) };
			conn_vector
				.lock()
				.unwrap()
				.last()
				.unwrap()
				.borrow()
				.send_command(msg)
		};

		let recv_thread = move || conn.recv_command();

		let s_handle = thread::spawn(send_thread);
		let r_handle = thread::spawn(recv_thread);

		let _s_res = s_handle.join().unwrap().expect("Failed to send cmd");
		let r_res = r_handle.join().unwrap().expect("Failed to recv cmd");

		//assert!(s_res, size_of::<CommandMsg>());
		assert!(r_res.is_some());

		let mut cmp_msg = CommandMsg::default();
		unsafe { (*cmp_msg.data.find_img).image_name.fill(1) };
		let rec_msg = r_res.unwrap();

		assert_eq!(cmp_msg.tag, rec_msg.tag);
		assert_eq!(unsafe { cmp_msg.data.find_img.image_name }, unsafe {
			rec_msg.data.find_img.image_name
		});
	}

	#[test]
	fn ipc_result_msg() {
		let _ = fs::remove_file(SOCK_PATH);
		let (listener, conn) = _ipc_stream_create();

		let send_thread = move || {
			let mut msg = ResultMsg::default();
			unsafe { (*msg.data.find_img).img_data.height = 1024 };
			listener
				.connections
				.lock()
				.unwrap()
				.last()
				.unwrap()
				.borrow()
				.send_result(msg)
		};

		let recv_thread = move || conn.recv_result();

		let s_handle = thread::spawn(send_thread);
		let r_handle = thread::spawn(recv_thread);

		let _s_res = s_handle.join().unwrap().expect("Failed to send res");
		let r_res = r_handle.join().unwrap().expect("Failed to recv res");

		//assert_eq!(s_res, size_of::<ResultMsg>());
		assert!(r_res.is_some());

		let mut cmp_msg = ResultMsg::default();
		unsafe { (*cmp_msg.data.find_img).img_data.height = 1024 };
		let rec_msg = r_res.unwrap();

		assert_eq!(cmp_msg.tag, rec_msg.tag);
		assert_eq!(unsafe { cmp_msg.data.find_img.img_data.height }, unsafe {
			rec_msg.data.find_img.img_data.height
		});
	}

	#[test]
	fn ipc_ancillary() {
		let _ = fs::remove_file(SOCK_PATH);
		let (listener, conn) = _ipc_stream_create();

		let tmp1 = tempfile::NamedTempFile::new().unwrap();
		let tmp2 = tempfile::NamedTempFile::new().unwrap();

		let fd1 = tmp1.as_raw_fd();
		let fd2 = tmp2.as_raw_fd();

		let handles = [fd1.as_raw_fd(), fd2.as_raw_fd()];
		let handle_count = handles.len();

		let send_thread = move || {
			listener
				.connections
				.lock()
				.unwrap()
				.last()
				.unwrap()
				.borrow()
				.send_anillary_handles(&handles)
		};

		let recv_thread = move || conn.recv_ancillary(handle_count);

		let s_handle = thread::spawn(send_thread);
		let r_handle = thread::spawn(recv_thread);

		let mut r_res = r_handle.join().unwrap().expect("Failed to recv ancillary");
		let _s_res = s_handle.join().unwrap().expect("Failed to send ancillary");

		//assert_ne!(s_res, 0);
		assert_eq!(r_res.len(), handle_count);
		for fd in r_res.iter() {
			assert!(fd.as_raw_fd() >= 0);
		}

		r_res.clear();

		//tmp1.close().unwrap();
	}
}
