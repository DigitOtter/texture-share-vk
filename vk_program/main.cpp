#include "vk_program.h"

int main(int argc, char **argv)
{
	VkProgram program;
	program.Init();

	//main loop
	SDL_Event e;
	bool bQuit = false;
	while (!bQuit)
	{
		//Handle events on queue
		while (SDL_PollEvent(&e) != 0)
		{
			//close the window when user alt-f4s or clicks the X button
			if (e.type == SDL_QUIT) bQuit = true;
		}

		program.Draw();
	}

	program.Cleanup();
	return 0;
}
