#pragma once

extern "C" {
#include "texture_share_vk/texture_share_vk_client.h"
}
#include <cstdint>
#include <string_view>
#include <vulkan/vulkan_core.h>

class TextureShareVkClient
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

	TextureShareVkClient();
	~TextureShareVkClient();

	TextureShareVkClient(const TextureShareVkClient &)            = delete;
	TextureShareVkClient &operator=(const TextureShareVkClient &) = delete;
	TextureShareVkClient(TextureShareVkClient &&other);
	TextureShareVkClient &operator=(TextureShareVkClient &&other);

	bool init(VkSetup *vk_setup, const char *socket_path = DEFAULT_SOCKET_PATH.data(),
	          uint64_t timeout_in_millis = 1000);
	bool init_with_server_launch(
		VkSetup *vk_setup, const char *socket_path = DEFAULT_SOCKET_PATH.data(),
		uint64_t client_timeout_in_millis = 1000, const char *server_program = VK_SERVER_EXECUTABLE,
		const char *server_lock_path   = DEFAULT_LOCKFILE_PATH.data(),
		const char *server_socket_path = DEFAULT_SOCKET_PATH.data(),
		const char *shmem_prefix = DEFAULT_SHMEM_PREFIX.data(), uint64_t server_socket_timeout_in_millis = 2 * 1000,
		uint64_t server_connection_wait_timeout_in_millis = 2 * 1000, uint64_t server_ipc_timeout_in_millis = 2 * 1000,
		uint64_t server_lockfile_timeout_in_millis = 2 * 1000, uint64_t server_spawn_timeout_in_millis = 20 * 1000);

	void destroy_client();

	ImageLookupResult init_image(const char *image_name, uint32_t width, uint32_t height, ImgFormat format,
	                             bool overwrite_existing);

	ImageLookupResult find_image(const char *image_name, bool force_update);
	ClientImageDataGuard find_image_data(const char *image_name, bool force_update);

	int send_image(const char *image_name, VkImage image, VkImageLayout orig_layout, VkImageLayout target_layout,
	               VkFence fence, VkOffset3D *extents = nullptr);

	int recv_image(const char *image_name, VkImage image, VkImageLayout orig_layout, VkImageLayout target_layout,
	               VkFence fence, VkOffset3D *extents = nullptr);

	private:
	VkClient *_client = nullptr;
};
