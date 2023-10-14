#include "texture_share_gl_client.hpp"
#include "texture_share_gl/texture_share_gl_client.h"
#include <utility>

TextureShareGlClient::ClientImageDataGuard::ClientImageDataGuard(::ClientImageDataGuard *data)
	: _data(data)
{}

TextureShareGlClient::ClientImageDataGuard::~ClientImageDataGuard()
{
	gl_client_image_data_guard_destroy(this->_data);
	this->_data = nullptr;
}

TextureShareGlClient::ClientImageDataGuard::ClientImageDataGuard(ClientImageDataGuard &&other)
	: _data(std::move(other._data))
{
	other._data = nullptr;
}

TextureShareGlClient::ClientImageDataGuard &TextureShareGlClient::ClientImageDataGuard::operator=(
	ClientImageDataGuard &&other)
{
	this->_data = std::move(other._data);
	other._data = nullptr;

	return *this;
}

bool TextureShareGlClient::ClientImageDataGuard::is_valid() const
{
	return this->_data != nullptr;
}

const ShmemDataInternal *TextureShareGlClient::ClientImageDataGuard::read() const
{
	if(!this->_data)
		return nullptr;

	return gl_client_image_data_guard_read(this->_data);
}

TextureShareGlClient::TextureShareGlClient() {}

TextureShareGlClient::TextureShareGlClient::TextureShareGlClient(TextureShareGlClient &&other)
	: _client(std::move(other._client))
{
	other._client = nullptr;
}

TextureShareGlClient &TextureShareGlClient::TextureShareGlClient::operator=(TextureShareGlClient &&other)
{
	this->_client = std::move(other._client);
	other._client = nullptr;

	return *this;
}

TextureShareGlClient::~TextureShareGlClient()
{
	this->destroy_client();
}

bool TextureShareGlClient::initialize_gl_external()
{
	return gl_client_initialize_external_gl();
}

bool TextureShareGlClient::init(const char *socket_path, uint64_t timeout_in_millis)
{
	this->destroy_client();
	this->_client = gl_client_new(socket_path, timeout_in_millis);

	return this->_client != nullptr;
}

bool TextureShareGlClient::init_with_server_launch(const char *socket_path, uint64_t client_timeout_in_millis,
                                                   const char *server_program, const char *server_lock_path,
                                                   const char *server_socket_path, const char *shmem_prefix,
                                                   uint64_t server_connection_timeout_in_millia,
                                                   uint64_t server_spawn_timeout_in_millis)
{
	this->destroy_client();
	this->_client = gl_client_new_with_server_launch(
		socket_path, client_timeout_in_millis, server_program, server_lock_path, server_socket_path, shmem_prefix,
		server_connection_timeout_in_millia, server_spawn_timeout_in_millis);

	return this->_client != nullptr;
}

void TextureShareGlClient::destroy_client()
{
	gl_client_destroy(this->_client);
	this->_client = nullptr;
}

ImageLookupResult TextureShareGlClient::init_image(const char *image_name, uint32_t width, uint32_t height,
                                                   ImgFormat format, bool overwrite_existing)
{
	if(!this->_client)
		return ImageLookupResult::Error;

	return gl_client_init_image(this->_client, image_name, width, height, format, overwrite_existing);
}

ImageLookupResult TextureShareGlClient::find_image(const char *image_name, bool force_update)
{
	if(!this->_client)
		return ImageLookupResult::Error;

	return gl_client_find_image(this->_client, image_name, force_update);
}

TextureShareGlClient::ClientImageDataGuard TextureShareGlClient::find_image_data(const char *image_name,
                                                                                 bool force_update)
{
	if(!this->_client)
		return nullptr;

	return ClientImageDataGuard(gl_client_find_image_data(this->_client, image_name, force_update));
}

int TextureShareGlClient::send_image(const char *image_name, GLuint src_texture_id, GLenum src_texture_target,
                                     bool invert, GLuint prev_fbo, const struct ImageExtent *extents)
{
	if(!this->_client)
		return -1;

	return gl_client_send_image(this->_client, image_name, src_texture_id, src_texture_target, invert, prev_fbo,
	                            extents);
}

int TextureShareGlClient::recv_image(const char *image_name, GLuint dst_texture_id, GLenum dst_texture_target,
                                     bool invert, GLuint prev_fbo, const struct ImageExtent *extents)
{
	if(!this->_client)
		return -1;

	return gl_client_recv_image(this->_client, image_name, dst_texture_id, dst_texture_target, invert, prev_fbo,
	                            extents);
}
