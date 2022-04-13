#ifndef TEX_SHARE_LOGGING_H
#define TEX_SHARE_LOGGING_H

#include "VkBootstrap.h"

#include <iostream>


/// @brief Helper macro to test the result of Vulkan calls which can return an error.
#define VK_CHECK(x)                                                        \
	do                                                                     \
    {                                                                      \
	    VkResult err = x;                                                  \
	    if (err)                                                           \
        {                                                                  \
	        std::cerr << "Detected Vulkan error: " << err << std::endl;    \
	        abort();                                                       \
	    }                                                                  \
	} while(0);

#endif //TEX_SHARE_LOGGING_H
