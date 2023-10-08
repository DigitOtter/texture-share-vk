#include "gl_shared_image_wrapper.h"
#include "gl_shared_image/platform/linux/external_handle.h"
// #include "gl_shared_image/platform/linux/external_handle.h"
// #include "gl_shared_image/platform/linux/external_handle_gl.h"
#include <memory>

std::unique_ptr<GlSharedImageWrapper> gl_shared_image_new()
{
	return std::make_unique<GlSharedImageWrapper>();
}

std::unique_ptr<ShareHandlesWrapper> gl_share_handles_new()
{
	return std::make_unique<ShareHandlesWrapper>();
}

std::unique_ptr<ShareHandlesWrapper> gl_share_handles_from_fd(int memory_fd)
{
	ExternalHandle::ShareHandles handles;
	handles.memory = memory_fd;

	return std::make_unique<ShareHandlesWrapper>(std::move(handles));
}

bool gl_external_initialize()
{
	return GlSharedImageWrapper::InitializeGLExternal();
}
