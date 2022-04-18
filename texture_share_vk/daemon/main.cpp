#include "texture_share_vk/daemon/texture_share_daemon.h"


int main(int argc, char *argv[])
{
	TextureShareDaemon tex_share_d;

	tex_share_d.Initialize();

	const auto res = tex_share_d.Loop();

	const auto cleanup_res = tex_share_d.Cleanup();

	if(res >= 0)
		return cleanup_res;

	return res;
}
