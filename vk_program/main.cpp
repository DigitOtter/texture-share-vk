#include "vk_program.h"
#include "vk_test_image_share.h"

#include "texture_share_vk/shared_image_handle_vk.h"
#include "texture_share_vk/texture_share_vk_client.h"


int main(int argc, char **argv)
{
	VkTestImageShare program;
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
			if (e.type == SDL_QUIT)
				bQuit = true;
		}

		program.Draw();
	}

	program.Cleanup();
	return 0;
}

//int main(int argc, char **argv)
//{
//	TextureShareVkClient client;
//	client.InitializeVulkan();
//	//client.InitDaemon();

//	client.InitImage("test_image", 800, 600, VK_FORMAT_R8G8B8A8_UNORM);

//	VkClearColorValue img_clr{};
//	img_clr.float32[0] = 1.0f;
//	img_clr.float32[1] = 1.0f;
//	img_clr.float32[2] = 0.0f;
//	img_clr.float32[3] = 1.0f;

//	client.ClearImage(img_clr, VK_NULL_HANDLE);

//	client.CleanupVulkan();

//	return 0;
//}

//int main(int argc, char **argv)
//{
//	VkProgram program;
//	program.Init();

//	TextureShareVk tex_share_vk;
//	tex_share_vk.InitializeVulkan();

//	auto ext_handles = program.GetSharedImage().ExportHandles();
//	SharedImageHandleVk shared_image_handle = tex_share_vk.CreateImageHandle(std::move(ext_handles),
//	                                                                         program.GetSharedImage().image_width, program.GetSharedImage().image_height,
//	                                                                         program.GetSharedImage().image_format);

//	VkClearColorValue img_clr{};
//	img_clr.float32[0] = 1.0f;
//	img_clr.float32[1] = 1.0f;
//	img_clr.float32[2] = 0.0f;
//	img_clr.float32[3] = 1.0f;

//	shared_image_handle.ClearImage(tex_share_vk.GraphicsQueue(), tex_share_vk.CommandBuffer(), img_clr, VK_NULL_HANDLE);
//	sleep(1);

//	//main loop
//	SDL_Event e;
//	bool bQuit = false;
//	while (!bQuit)
//	{
//		//Handle events on queue
//		while (SDL_PollEvent(&e) != 0)
//		{
//			//close the window when user alt-f4s or clicks the X button
//			if (e.type == SDL_QUIT)
//				bQuit = true;
//		}

//		program.Draw();
//	}

//	shared_image_handle.Cleanup();
//	tex_share_vk.CleanupVulkan();

//	program.Cleanup();
//	return 0;
//}
