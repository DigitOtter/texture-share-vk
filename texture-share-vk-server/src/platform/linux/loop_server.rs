use polling::{Event, Events, PollMode, Poller};
use std::os::fd::{AsFd, AsRawFd};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::SystemTime;



use crate::VkServer;

impl VkServer {
	pub fn loop_server(
		mut self,
		stop_bit: Arc<AtomicBool>,
	) -> Result<(), Box<dyn std::error::Error>> {
		// Stop server if no connection was established after NO_CONNECTION_TIMEOUT
		let mut conn_timeout = SystemTime::now() + self.connection_wait_timeout;

		// Setup polling
		let mut new_connection_waiting = false;
		let poller = Poller::new()?;
		let mut events = Events::new();
		let mut connections_to_close = Vec::default();

		// Add listener event request to poller
		unsafe {
			poller.add_with_mode(
				self.socket.get_socket(),
				Event::readable(VkServer::LISTENER_EVENT_KEY),
				PollMode::Level,
			)?;
		};

		loop {
			if new_connection_waiting || !connections_to_close.is_empty() {
				{
					let mut conn_lock = self.socket.connections.lock();
					// Remove all connections from poller
					for conn_id in 0..conn_lock.as_ref().unwrap().len() {
						poller.delete(
							conn_lock.as_ref().unwrap()[conn_id]
								.borrow()
								.get_socket()
								.as_fd(),
						)?;
					}

					// Remove unused connections from both poller and connections vector
					if !connections_to_close.is_empty() {
						// Remove connections that were closed by peer
						for ci in connections_to_close.iter().rev() {
							conn_lock.as_mut().unwrap().remove(*ci);
						}

						connections_to_close.clear();
					}
				}

				if new_connection_waiting {
					// Accept event received
					self.socket.try_accept()?;
					new_connection_waiting = false;
				}

				// Add poll request for each connection
				let conn_lock = self.socket.connections.lock();
				for conn_id in 0..conn_lock.as_ref().unwrap().len() {
					unsafe {
						poller.add(
							conn_lock.as_ref().unwrap()[conn_id]
								.borrow()
								.get_socket()
								.as_raw_fd(),
							Event::readable(conn_id).with_interrupt(),
						)?;
					}
				}
			};

			events.clear();
			poller.wait(&mut events, Some(self.socket.timeout))?;

			for ev in events.iter() {
				if ev.key < VkServer::LISTENER_EVENT_KEY {
					let conn_lock = self.socket.connections.lock();
					let connections = conn_lock.as_ref().unwrap();
					// Close connection if socket was closed
					if ev.is_interrupt() {
						connections_to_close.push(ev.key);
						continue;
					} else {
						let conn = &connections[ev.key];
						if !VkServer::process_single_connection(
							&conn.borrow(),
							&self.vk_instance,
							&mut self.vk_devices,
							&self.shmem_prefix,
							&mut self.images,
							self.ipc_timeout,
						)? {
							connections_to_close.push(ev.key);
						}

						poller.modify(
							conn.borrow().get_socket().as_fd(),
							Event::readable(ev.key).with_interrupt(),
						)?;
					}
				} else if ev.key == VkServer::LISTENER_EVENT_KEY {
					poller.modify(
						self.socket.get_socket(),
						Event::readable(VkServer::LISTENER_EVENT_KEY),
					)?;
					new_connection_waiting = true;
				}
			}

			// Stop if no connections active
			if self.socket.connections.lock().as_ref().unwrap().is_empty() {
				if SystemTime::now() > conn_timeout {
					//println!("No connections active. Closing server...");
					break;
				}
			} else {
				conn_timeout = SystemTime::now() + self.connection_wait_timeout;
			}

			// Break if externally requested
			if stop_bit.load(Ordering::Relaxed) {
				break;
			}
		}

		poller.delete(self.socket.get_socket().as_fd())?;

		Ok(())
	}
}
