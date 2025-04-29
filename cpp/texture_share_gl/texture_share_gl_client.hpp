#pragma once

extern "C" {
#include "texture_share_gl/texture_share_gl_client.h"
}
#include <string_view>

class TextureShareGlClient
{
	public:
	struct ClientImageDataGuard
	{
		ClientImageDataGuard(::ClientImageDataGuard *data);
		~ClientImageDataGuard();

		ClientImageDataGuard(const ClientImageDataGuard &)            = delete;
		ClientImageDataGuard &operator=(const ClientImageDataGuard &) = delete;
		ClientImageDataGuard(ClientImageDataGuard &&);
		ClientImageDataGuard &operator=(ClientImageDataGuard &&);

		bool is_valid() const;
		const ShmemDataInternal *read() const;

		private:
		::ClientImageDataGuard *_data = nullptr;
	};

	static constexpr std::string_view DEFAULT_SHMEM_PREFIX  = VK_SERVER_DEFAULT_SHMEM_PREFIX;
	static constexpr std::string_view DEFAULT_LOCKFILE_PATH = VK_SERVER_DEFAULT_LOCKFILE_PATH;
	static constexpr std::string_view DEFAULT_SOCKET_PATH   = VK_SERVER_DEFAULT_SOCKET_PATH;

	TextureShareGlClient();
	~TextureShareGlClient();

	TextureShareGlClient(const TextureShareGlClient &)            = delete;
	TextureShareGlClient &operator=(const TextureShareGlClient &) = delete;
	TextureShareGlClient(TextureShareGlClient &&other);
	TextureShareGlClient &operator=(TextureShareGlClient &&other);

	static bool initialize_gl_external();

	bool init(const char *socket_path = DEFAULT_SOCKET_PATH.data(), uint64_t timeout_in_millis = 1000);
	bool init_with_server_launch(
		const char *socket_path = DEFAULT_SOCKET_PATH.data(), uint64_t client_timeout_in_millis = 1000,
		const char *server_program = VK_SERVER_EXECUTABLE, const char *server_lock_path = DEFAULT_LOCKFILE_PATH.data(),
		const char *server_socket_path = DEFAULT_SOCKET_PATH.data(),
		const char *shmem_prefix = DEFAULT_SHMEM_PREFIX.data(), uint64_t server_socket_timeout_in_millis = 2 * 1000,
		uint64_t server_connection_wait_timeout_in_millis = 2 * 1000, uint64_t server_ipc_timeout_in_millis = 2 * 1000,
		uint64_t server_lockfile_timeout_in_millis = 2 * 1000, uint64_t server_spawn_timeout_in_millis = 20 * 1000);

	void destroy_client();

	ImageLookupResult init_image(const char *image_name, uint32_t width, uint32_t height, ImgFormat format,
	                             bool overwrite_existing);

	ImageLookupResult find_image(const char *image_name, bool force_update);
	ClientImageDataGuard find_image_data(const char *image_name, bool force_update);

	int send_image(const char *image_name, GLuint src_texture_id, GLenum src_texture_target, bool invert,
	               GLuint prev_fbo, const struct GlImageExtent *extents);

	int recv_image(const char *image_name, GLuint dst_texture_id, GLenum dst_texture_target, bool invert,
	               GLuint prev_fbo, const struct GlImageExtent *extents);

	private:
	struct GlClient *_client = nullptr;
};
