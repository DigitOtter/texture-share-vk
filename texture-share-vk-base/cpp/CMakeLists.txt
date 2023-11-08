set(PROJECT_NAME "VkSharedImage")
set(HEADER_DIRECTORY "vk_shared_image")

set(NAMESPACE_NAME "${PROJECT_NAME}")

set(LIBRARY_NAME "${PROJECT_NAME}")
set(EXECUTABLE_NAME "${PROJECT_NAME}Exec")
set(TEST_NAME "${PROJECT_NAME}Tests")

set(LIB_EXPORT_NAME "${LIBRARY_NAME}Targets")
set(LIB_CONFIG_NAME "${LIBRARY_NAME}Config")
set(LIB_VERSION_NAME "${LIB_CONFIG_NAME}Version")

cmake_minimum_required(VERSION 3.18)
project("${PROJECT_NAME}" VERSION 1.0.0)

include(GNUInstallDirs)

set(CMAKE_EXPORT_COMPILE_COMMANDS ON)
set(CMAKE_CXX_STANDARD 20)

find_package(Vulkan REQUIRED)
# find_package(OpenGL REQUIRED)

set(CMAKE_POSITION_INDEPENDENT_CODE ON)

# By default, don't install VkBootstrap
option(INSTALL_VK_BOOTSTRAP "Install vk-bootstrap" OFF)
if(NOT ${INSTALL_VK_BOOTSTRAP})
	set(IVK EXCLUDE_FROM_ALL)
else()
	set(IVK)
endif()

# Build VkBootstrap as static library
set(TMP_BSL ${BUILD_SHARED_LIBS})
set(BUILD_SHARED_LIBS OFF)
add_subdirectory("third_party/vk-bootstrap" ${IVK})
set(BUILD_SHARED_LIBS ${TMP_BSL})

add_subdirectory("third_party/volk")

# Turn string passed with environment variable back to cmake list
string(REPLACE "," ";" TSV_PLATFORM_VK_SRC      ${TSV_PLATFORM_VK_SRC})

# cmake-format: off
set(LIB_SRC_FILES
	"vk_shared_image/vk_helpers.cpp"
	"vk_shared_image/vk_setup.cpp"
	"vk_shared_image/vk_shared_image.cpp"
	${TSV_PLATFORM_VK_SRC}
)
# cmake-format: on

# ##############################################################################################################################################################
# Library
add_library("${LIBRARY_NAME}" ${LIB_SRC_FILES})
add_library("${NAMESPACE_NAME}::${LIBRARY_NAME}" ALIAS ${LIBRARY_NAME})
target_compile_options(${LIBRARY_NAME} PUBLIC $<$<OR:$<CXX_COMPILER_ID:Clang>,$<CXX_COMPILER_ID:GNU>>:-Wall -Wextra>)

target_include_directories(
	${LIBRARY_NAME}
	PUBLIC 
		"$<INSTALL_INTERFACE:${CMAKE_INSTALL_INCLUDEDIR}>"
		"$<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}>"
		"$<BUILD_INTERFACE:${TSV_PLATFORM_INCLUDE_DIR}>"
	PRIVATE
)

target_link_libraries(
	${LIBRARY_NAME}
	PUBLIC 
		Vulkan::Vulkan
	PRIVATE 
		vk-bootstrap::vk-bootstrap
)

# #
# ##############################################################################################################################################################
# # Install library files set(INSTALL_CONFIGDIR "${CMAKE_INSTALL_LIBDIR}/cmake/${PROJECT_NAME}") install( TARGETS ${LIBRARY_NAME} EXPORT ${LIB_EXPORT_NAME}
# LIBRARY DESTINATION ${CMAKE_INSTALL_LIBDIR} ARCHIVE DESTINATION ${CMAKE_INSTALL_LIBDIR} RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR} PUBLIC_HEADER DESTINATION
# ${CMAKE_INSTALL_INCLUDEDIR}/${HEADER_DIRECTORY} )

# # Install headers install( DIRECTORY ${HEADER_DIRECTORY} DESTINATION "${CMAKE_INSTALL_INCLUDEDIR}/${HEADER_DIRECTORY}" FILES_MATCHING PATTERN "*.h" PATTERN
# "*.hpp" )

# # Export target install( EXPORT ${LIB_EXPORT_NAME} DESTINATION ${INSTALL_CONFIGDIR} FILE "${LIB_EXPORT_NAME}.cmake" NAMESPACE "${NAMESPACE_NAME}::" )

# # create version and config files include(CMakePackageConfigHelpers) write_basic_package_version_file( "${CMAKE_CURRENT_BINARY_DIR}/${LIB_VERSION_NAME}.cmake"
# VERSION ${PROJECT_VERSION} COMPATIBILITY AnyNewerVersion )

# configure_package_config_file( "${CMAKE_CURRENT_LIST_DIR}/cmake/ProjectConfig.cmake.in" "${CMAKE_CURRENT_BINARY_DIR}/${LIB_CONFIG_NAME}.cmake"
# INSTALL_DESTINATION ${INSTALL_CONFIGDIR} )

# # Install all the helper files install(FILES "${CMAKE_CURRENT_BINARY_DIR}/${LIB_CONFIG_NAME}.cmake" "${CMAKE_CURRENT_BINARY_DIR}/${LIB_VERSION_NAME}.cmake"
# DESTINATION ${INSTALL_CONFIGDIR})
