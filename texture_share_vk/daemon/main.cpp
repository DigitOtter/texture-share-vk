#include "texture_share_vk/daemon/texture_share_daemon.h"

#include <signal.h>


volatile bool run = false;

void signalHandler(int signal)
{
	if(signal == SIGINT)
		run = false;
}

int main(int argc, char *argv[])
{
	signal(SIGINT, &signalHandler);
	run = false;

	TextureShareDaemon tex_share_d;

	tex_share_d.Initialize();

	const auto res = tex_share_d.Loop(run);

	const auto cleanup_res = tex_share_d.Cleanup();

	if(res >= 0)
		return cleanup_res;

	return res;
}
