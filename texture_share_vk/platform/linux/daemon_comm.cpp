#include "daemon_comm.h"

#include "texture_share_vk/ipc_memory.h"
#include "texture_share_vk/platform/config.h"

#include <sys/socket.h>
#include <sys/un.h>
#include <thread>
#include <unistd.h>


constexpr size_t EXT_HANDLE_CMSG_LEN = sizeof(ExternalHandle::TYPE)*3;

/* Ancillary data buffer, wrapped in a union
 * in order to ensure it is suitably aligned */
union CmsgData
{
	char buf[CMSG_SPACE(EXT_HANDLE_CMSG_LEN)] = {};
	struct cmsghdr align;
};

void DaemonComm::Daemonize(const std::string &ipc_cmd_memory_segment, const std::string &ipc_map_memory_segment)
{
	if(IpcMemory::SharedMemoryExists(ipc_cmd_memory_segment))
		return;

	int c_pid = fork();
	if(c_pid == 0)
	{
		// Child process
		if(setsid() < 0)
			throw std::runtime_error("Failed to daemonize texture share daemon");

		const int ret = execlp(TSV_DAEMON_PATH,
		                       ipc_cmd_memory_segment.c_str(),
		                       ipc_map_memory_segment.c_str(),
		                       nullptr);
		exit(ret);
	}
	else if(c_pid < 0)
	{
		throw std::runtime_error("Failed to create texture share daemon");
	}
}

void DaemonComm::SendHandles(ExternalHandle::ShareHandles &&handles, const std::filesystem::path &socket_path, uint64_t micro_sec_wait_time)
{
	// Create socket
	NamedSock sock_fd(socket_path, socket(AF_UNIX, SOCK_STREAM | SOCK_NONBLOCK, 0));
	DaemonComm::CreateNamedUnixSocket(socket_path, sock_fd);

	// Wait for receiver connect
	FileDesc conn_fd = DaemonComm::AcceptNamedUnixSocket(sock_fd, micro_sec_wait_time);

	// Create file descriptor send message
	// Uses SCM_RIGHTS to transfer file descriptors between processes
	// Code from 'man 2 seccomp_unotify' 'sendfd'
	struct msghdr msgh;
	struct iovec iov;
	int data;
	struct cmsghdr *cmsgp;

	/* Allocate a char array of suitable size to hold the ancillary data.
	  However, since this buffer is in reality a 'struct cmsghdr', use a
	  union to ensure that it is suitably aligned. */
	CmsgData controlMsg;

	/* The 'msg_name' field can be used to specify the address of the
	  destination socket when sending a datagram. However, we do not
	  need to use this field because 'sockfd' is a connected socket. */

	msgh.msg_name = NULL;
	msgh.msg_namelen = 0;

	/* On Linux, we must transmit at least one byte of real data in
	  order to send ancillary data. We transmit an arbitrary integer
	  whose value is ignored by recvfd(). */
	msgh.msg_iov = &iov;
	msgh.msg_iovlen = 1;
	iov.iov_base = &data;
	iov.iov_len = sizeof(int);
	data = 12345;

	/* Set 'msghdr' fields that describe ancillary data */
	msgh.msg_control = controlMsg.buf;
	msgh.msg_controllen = sizeof(controlMsg.buf);

	/* Set up ancillary data describing file descriptor to send */
	cmsgp = CMSG_FIRSTHDR(&msgh);
	cmsgp->cmsg_level = SOL_SOCKET;
	cmsgp->cmsg_type = SCM_RIGHTS;
	cmsgp->cmsg_len = CMSG_LEN(EXT_HANDLE_CMSG_LEN);

	int myfds[3] = {handles.memory, handles.ext_read, handles.ext_write};  /* Contains the file descriptors to pass */
	memcpy(CMSG_DATA(cmsgp), &myfds, sizeof(myfds));

	/* Send real plus ancillary data */
	int nr;

	const auto chrono_wait_time = std::chrono::microseconds(micro_sec_wait_time)/10;
	const auto stop_time = std::chrono::high_resolution_clock::now() + std::chrono::microseconds(micro_sec_wait_time);
	do
	{
		nr = sendmsg(conn_fd, &msgh, 0);
		if(nr == -1)
		{
			// Wait for receiver to connect (Error code 107 indicates a missing endpoint). Stop at other error
			if(errno == 107)
				std::this_thread::sleep_for(chrono_wait_time);
			else
				break;
		}
		else
			break;
	}
	while(std::chrono::high_resolution_clock::now() <= stop_time);

	if(nr == -1)
		throw std::runtime_error("Socket '" + socket_path.string() + "' encountered error on send: " + std::to_string(errno) + "\n\t" + strerror(errno));

	// Give receiver time to get message before closing socket
	std::this_thread::sleep_for(std::chrono::microseconds(micro_sec_wait_time));
}

ExternalHandle::ShareHandles DaemonComm::RecvHandles(const std::filesystem::path &socket_path, uint64_t micro_sec_wait_time)
{
	// Create non-blocking socket
	FileDesc conn_fd = socket(AF_UNIX, SOCK_STREAM | SOCK_NONBLOCK, 0);
	DaemonComm::ConnectNamedUnixSocket(socket_path, conn_fd);

	// Receive fd. Code from 'man 2 seccomp_unotify' 'recvfd'
	struct msghdr msgh;
	struct iovec iov;
	int data;
	ssize_t nr;

	/* Allocate a char buffer for the ancillary data. See the comments
	in sendfd() */
	CmsgData controlMsg;
	struct cmsghdr *cmsgp;

	/* The 'msg_name' field can be used to obtain the address of the
	sending socket. However, we do not need this information. */

	msgh.msg_name = NULL;
	msgh.msg_namelen = 0;

	/* Specify buffer for receiving real data */

	msgh.msg_iov = &iov;
	msgh.msg_iovlen = 1;
	iov.iov_base = &data;       /* Real data is an 'int' */
	iov.iov_len = sizeof(int);

	/* Set 'msghdr' fields that describe ancillary data */

	msgh.msg_control = controlMsg.buf;
	msgh.msg_controllen = sizeof(controlMsg.buf);

	/* Receive real plus ancillary data; real data is ignored */
	const auto chrono_wait_time = std::chrono::microseconds(micro_sec_wait_time)/10;
	const auto stop_time = std::chrono::high_resolution_clock::now() + std::chrono::microseconds(micro_sec_wait_time);
	do
	{
		nr = recvmsg(conn_fd, &msgh, 0);
		if(nr == -1)
		{
			// Stop loop at unexpected error
			if(errno == EWOULDBLOCK || errno == EAGAIN)
				std::this_thread::sleep_for(chrono_wait_time);
			else
				break;
		}
		else
			break;
	}
	while(std::chrono::high_resolution_clock::now() <= stop_time);

	if (nr == -1)
	{
		if(errno == EWOULDBLOCK || errno == EAGAIN)
			throw std::runtime_error("Socket '" + socket_path.string() + "' timed out while waiting for image fd");
		else
			throw std::runtime_error("Socket '" + socket_path.string() + "' encountered error on receive: " + std::to_string(errno) + "\n\t" + strerror(errno));
	}

	cmsgp = CMSG_FIRSTHDR(&msgh);

	/* Check the validity of the 'cmsghdr' */
	if (cmsgp == NULL ||
	    cmsgp->cmsg_len != CMSG_LEN(EXT_HANDLE_CMSG_LEN) ||
	    cmsgp->cmsg_level != SOL_SOCKET ||
	    cmsgp->cmsg_type != SCM_RIGHTS)
	{
	   throw std::runtime_error("Received invalid socket fd data");
	}

	const int *rec_fd = (const int*)CMSG_DATA(cmsgp);
	ExternalHandle::ShareHandles handles;
	handles.memory = rec_fd[0];
	handles.ext_read = rec_fd[1];
	handles.ext_write = rec_fd[2];

	return handles;
}

DaemonComm::FileDesc::FileDesc(int fd)
    : _fd(fd)
{}

DaemonComm::FileDesc::~FileDesc()
{
	if(this->_fd >= 0)
	{
		close(this->_fd);
		this->_fd = -1;
	}
}

DaemonComm::NamedSock::NamedSock(const std::filesystem::path &socket_path, int fd)
    : FileDesc(fd),
      _socket_path(socket_path)
{}

DaemonComm::NamedSock::~NamedSock()
{
	if((int)*this >= 0)
	{
		const std::filesystem::directory_entry dir_entry(this->_socket_path);
		if(dir_entry.exists() && dir_entry.is_socket())
			std::filesystem::remove(this->_socket_path);
	}
}

int DaemonComm::CreateNamedUnixSocket(const std::filesystem::path &socket_path, FileDesc &sock_fd)
{
	// Create socket
	struct sockaddr_un named_socket;
	memset(&named_socket, 0, sizeof(named_socket));

	const char *socket_name = socket_path.c_str();
	if(strlen(socket_name) >= sizeof(named_socket.sun_path) -1)
		throw std::runtime_error("Socket name '" + socket_path.string() + "' too large");

	named_socket.sun_family = AF_UNIX;
	strcpy(named_socket.sun_path, (char *)socket_path.c_str());
	if(bind(sock_fd, (struct sockaddr *)&named_socket, sizeof(struct sockaddr_un)) < 0)
		throw std::runtime_error("Socket name '" + socket_path.string() + "' failed to bind:" + std::to_string(errno) + "\n\t" + strerror(errno));

	if(listen(sock_fd, 10) < 0)
		throw std::runtime_error("Socket name '" + socket_path.string() + "' failed to listen: " + std::to_string(errno) + "\n\t" + strerror(errno));

	return sock_fd;
}

int DaemonComm::AcceptNamedUnixSocket(const FileDesc &sock_fd, uint64_t micro_sec_wait_time)
{
	int conn_fd;

	const auto chrono_wait_time = std::chrono::microseconds(micro_sec_wait_time)/10;
	const auto stop_time = std::chrono::high_resolution_clock::now() + std::chrono::microseconds(micro_sec_wait_time);
	do
	{
		conn_fd = accept(sock_fd, nullptr, nullptr);
		if(conn_fd < 0)
		{
			// Stop loop at unexpected error
			if(errno == EWOULDBLOCK || errno == EAGAIN)
				std::this_thread::sleep_for(chrono_wait_time);
			else
				break;
		}
		else
			break;
	}
	while(std::chrono::high_resolution_clock::now() <= stop_time);

	if (conn_fd < -1)
	{
		if(errno == EWOULDBLOCK || errno == EAGAIN)
			throw std::runtime_error("Socket timed out while waiting to accept connection");
		else
			throw std::runtime_error("Socket encountered error while waiting to accept connection: " + std::to_string(errno) + "\n\t" + strerror(errno));
	}

	return conn_fd;
}

int DaemonComm::ConnectNamedUnixSocket(const std::filesystem::path &socket_path, FileDesc &sock_fd, uint64_t micro_sec_wait_time)
{
	// Connect socket
	struct sockaddr_un named_socket;
	memset(&named_socket, 0, sizeof(named_socket));

	const char *socket_name = socket_path.c_str();
	if(strlen(socket_name) >= sizeof(named_socket.sun_path) -1)
		throw std::runtime_error("Socket name '" + socket_path.string() + "' too large");

	named_socket.sun_family = AF_UNIX;
	strcpy(named_socket.sun_path, (char *)socket_path.c_str());

	int nr;

	const auto chrono_wait_time = std::chrono::microseconds(micro_sec_wait_time)/10;
	const auto stop_time = std::chrono::high_resolution_clock::now() + std::chrono::microseconds(micro_sec_wait_time);
	do
	{
		nr = connect(sock_fd, (struct sockaddr *)&named_socket, sizeof(struct sockaddr_un));;
		if(nr == -1)
			std::this_thread::sleep_for(chrono_wait_time);
		else
			break;
	}
	while(std::chrono::high_resolution_clock::now() <= stop_time);

	if(nr == -1)
		throw std::runtime_error("Socket '" + socket_path.string() + "' failed to connect with error: " + std::to_string(errno) + "\n\t" + strerror(errno));

	return sock_fd;
}
