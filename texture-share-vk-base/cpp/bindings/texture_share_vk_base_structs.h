#pragma once

#ifndef VK_CHECK
#define VK_CHECK(x)                                                     \
	do                                                                  \
	{                                                                   \
		VkResult err = x;                                               \
		if(err)                                                         \
		{                                                               \
			std::cerr << "Detected Vulkan error: " << err << std::endl; \
			abort();                                                    \
		}                                                               \
	}                                                                   \
	while(0);
#endif

struct VkSetup;
