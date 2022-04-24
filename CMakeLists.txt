set(PROJECT_NAME "TextureShareVulkan")
set(HEADER_DIRECTORY "texture_share_vk")

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

option(BUILD_EXECUTABLES "Build executables" OFF)

set (CMAKE_CXX_STANDARD 20)

find_package(Vulkan REQUIRED)
find_package(Boost REQUIRED COMPONENTS date_time)
find_package(OpenGL REQUIRED)

set(CMAKE_POSITION_INDEPENDENT_CODE ON)

add_subdirectory("third_party/vk-bootstrap")
add_subdirectory("third_party/volk")


if(WIN32)
	set(PLATFORM_DIRECTORY "win32")
else()
	set(PLATFORM_DIRECTORY "linux")
endif()


set(EXTERNAL_HANDLE_LIB_SRC_FILES
	"texture_share_vk/platform/${PLATFORM_DIRECTORY}/external_handle.cpp"
)

set(IPC_MEMORY_LIB_SRC_FILES
	"texture_share_vk/ipc_memory.cpp"
	"texture_share_vk/platform/${PLATFORM_DIRECTORY}/daemon_comm.cpp"
)

set(OPENGL_LIB_SRC_FILES
	"texture_share_vk/opengl/shared_image_handle_gl.cpp"
	"texture_share_vk/opengl/texture_share_gl_client.cpp"
	"texture_share_vk/platform/${PLATFORM_DIRECTORY}/external_handle_gl.cpp"
)

set(LIB_SRC_FILES
	"texture_share_vk/platform/${PLATFORM_DIRECTORY}/external_handle_vk.cpp"
	"texture_share_vk/shared_image_handle_vk.cpp"
	"texture_share_vk/shared_image_vk.cpp"
	"texture_share_vk/texture_share_vk.cpp"
	"texture_share_vk/texture_share_vk_client.cpp"
	"texture_share_vk/vk_helpers.cpp"
)

set(VK_D_SRC_FILES
	"texture_share_vk/daemon/ipc_memory_processor_vk.cpp"
	"texture_share_vk/daemon/texture_share_daemon.cpp"
	"texture_share_vk/daemon/main.cpp"
)

set(EXEC_SRC_FILES
	"vk_program/main.cpp"
	"vk_program/vk_test_image_share.cpp"
	"vk_program/vk_program.cpp"
)

set(TEST_GL_SRC_FILES
	"gl_program/gl_test_image_share.cpp"
	"gl_program/main.cpp"
)

set(TEST_SRC_FILES
)

set(TSV_COMPILE_OPTIONS $<$<OR:$<CXX_COMPILER_ID:Clang>,$<CXX_COMPILER_ID:GNU>>:-Wall -Wextra -Wno-missing-field-initializers>)

##########################################
## Configure header
set(TSV_DAEMON_NAME "TSVSharedImagesDaemon")
if(NOT WIN32)
	set(TSV_DAEMON_DIR "${CMAKE_INSTALL_FULL_BINDIR}")
	set(TSV_DAEMON_SOCKET_DIR "${CMAKE_INSTALL_FULL_RUNSTATEDIR}/${TSV_DAEMON_NAME}" CACHE PATH "Path to daemon socket directory")
	set(TSV_DAEMON_LOCK_FILE  "/tmp/${TSV_DAEMON_NAME}.lock" CACHE FILEPATH "Daemon lock file")
else()
	if(SIZEOF_VOID_P_CODE IS 4)
		set(TSV_DAEMON_DIR "${CMAKE_INSTALL_PREFIX}/bin/32bit")
	else()
		set(TSV_DAEMON_DIR "${CMAKE_INSTALL_PREFIX}/bin/64bit")
	endif()
endif()

configure_file("texture_share_vk/platform/config.h.in" "${CMAKE_BINARY_DIR}/include/texture_share_vk/platform/config.h" @ONLY)


##########################################
## External Header Library
add_library(ExternalHeader STATIC ${EXTERNAL_HANDLE_LIB_SRC_FILES})
add_library("${NAMESPACE_NAME}::ExternalHeader" ALIAS ExternalHeader)
target_compile_options(ExternalHeader PUBLIC ${TSV_COMPILE_OPTIONS})

target_include_directories(ExternalHeader
	PUBLIC
		"$<INSTALL_INTERFACE:${CMAKE_INSTALL_INCLUDEDIR}>"
		"$<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}>"
		"$<BUILD_INTERFACE:${CMAKE_BINARY_DIR}/include>"

	PRIVATE
)

##########################################
## IPC Memory Library
add_library(TSVIpcMemory SHARED ${IPC_MEMORY_LIB_SRC_FILES})
add_library("${NAMESPACE_NAME}::TSVIpcMemory" ALIAS TSVIpcMemory)
target_compile_options(TSVIpcMemory PUBLIC ${TSV_COMPILE_OPTIONS})

target_include_directories(TSVIpcMemory
	PUBLIC
		"$<INSTALL_INTERFACE:${CMAKE_INSTALL_INCLUDEDIR}>"
		"$<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}>"
		"$<BUILD_INTERFACE:${CMAKE_BINARY_DIR}/include>"

	PRIVATE
)

target_link_libraries(TSVIpcMemory
	PUBLIC
		Boost::boost
		Boost::date_time

	PRIVATE
		"${NAMESPACE_NAME}::ExternalHeader"
)

##########################################
## Vulkan Library
add_library("${LIBRARY_NAME}" SHARED ${LIB_SRC_FILES})
add_library("${NAMESPACE_NAME}::${LIBRARY_NAME}" ALIAS ${LIBRARY_NAME})
target_compile_options(${LIBRARY_NAME} PUBLIC ${TSV_COMPILE_OPTIONS})

target_include_directories(${LIBRARY_NAME}
	PUBLIC
		"$<INSTALL_INTERFACE:${CMAKE_INSTALL_INCLUDEDIR}>"
		"$<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}>"
		"$<BUILD_INTERFACE:${CMAKE_BINARY_DIR}/include>"

	PRIVATE
		"${NAMESPACE_NAME}::ExternalHeader"
)

target_link_libraries(${LIBRARY_NAME}
	PUBLIC
		Vulkan::Vulkan

		"${NAMESPACE_NAME}::TSVIpcMemory"

	PRIVATE
		vk-bootstrap::vk-bootstrap
		"${NAMESPACE_NAME}::ExternalHeader"
)

##########################################
## OpenGL Library
add_library(TextureShareOpenGL SHARED ${OPENGL_LIB_SRC_FILES})
add_library("${NAMESPACE_NAME}::TextureShareOpenGL" ALIAS TextureShareOpenGL)
target_compile_options(TextureShareOpenGL PUBLIC ${TSV_COMPILE_OPTIONS})


target_include_directories(TextureShareOpenGL
	PUBLIC
		"$<INSTALL_INTERFACE:${CMAKE_INSTALL_INCLUDEDIR}>"
		"$<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}>"
		"$<BUILD_INTERFACE:${CMAKE_BINARY_DIR}/include>"

	PRIVATE
		"${NAMESPACE_NAME}::ExternalHeader"
)

target_link_libraries(TextureShareOpenGL
	PUBLIC
		OpenGL::OpenGL

		"${NAMESPACE_NAME}::TSVIpcMemory"

	PRIVATE
		"${NAMESPACE_NAME}::ExternalHeader"
)


##########################################
## Shared Images Daemon
add_executable(TSVDaemon ${VK_D_SRC_FILES})
target_compile_options(TSVDaemon PUBLIC ${TSV_COMPILE_OPTIONS})

set_target_properties(TSVDaemon PROPERTIES OUTPUT_NAME ${TSV_DAEMON_NAME})

target_link_libraries(TSVDaemon
	PUBLIC
		"${NAMESPACE_NAME}::${LIBRARY_NAME}"
		"${NAMESPACE_NAME}::TSVIpcMemory"
		"${NAMESPACE_NAME}::ExternalHeader"
)



if(BUILD_EXECUTABLES)
	find_package(SDL2 REQUIRED)
	find_package(GLUT REQUIRED)

	##########################################
	## Test Vulkan Executable
	if(NOT "${EXEC_SRC_FILES}" STREQUAL "")
		add_executable("${EXECUTABLE_NAME}" ${EXEC_SRC_FILES})
		target_link_libraries("${EXECUTABLE_NAME}"
			${LIBRARY_NAME}
			SDL2::SDL2
			vk-bootstrap::vk-bootstrap
		)
	endif()


	##########################################
	## Test OpenGL Image sharing
	add_executable(TestGLImageShare ${TEST_GL_SRC_FILES})
	target_compile_options(TestGLImageShare PUBLIC ${TSV_COMPILE_OPTIONS})

	target_link_libraries(TestGLImageShare
		PUBLIC
			"${NAMESPACE_NAME}::TextureShareOpenGL"

			GLUT::GLUT
	)
endif()


##########################################
## Install library files
set(INSTALL_CONFIGDIR "${CMAKE_INSTALL_LIBDIR}/cmake/${PROJECT_NAME}")

install(TARGETS ${LIBRARY_NAME} TSVIpcMemory TextureShareOpenGL
	EXPORT ${LIB_EXPORT_NAME}
	LIBRARY DESTINATION ${CMAKE_INSTALL_LIBDIR}
	ARCHIVE DESTINATION ${CMAKE_INSTALL_LIBDIR}
	RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR}

	PUBLIC_HEADER DESTINATION ${CMAKE_INSTALL_INCLUDEDIR}/${HEADER_DIRECTORY})

# Install headers
install(DIRECTORY ${HEADER_DIRECTORY} "${CMAKE_BINARY_DIR}/include/${HEADER_DIRECTORY}"
	DESTINATION "${CMAKE_INSTALL_INCLUDEDIR}"
	FILES_MATCHING
		PATTERN "*.h"
		PATTERN "*.hpp")

# Export target
install(EXPORT ${LIB_EXPORT_NAME} 
	DESTINATION ${INSTALL_CONFIGDIR}
	FILE "${LIB_EXPORT_NAME}.cmake"
	NAMESPACE "${NAMESPACE_NAME}::")

# create version and config files
include(CMakePackageConfigHelpers)
write_basic_package_version_file("${CMAKE_CURRENT_BINARY_DIR}/${LIB_VERSION_NAME}.cmake"
	VERSION ${PROJECT_VERSION}
	COMPATIBILITY AnyNewerVersion)

configure_package_config_file("${CMAKE_CURRENT_LIST_DIR}/cmake/ProjectConfig.cmake.in"
	"${CMAKE_CURRENT_BINARY_DIR}/${LIB_CONFIG_NAME}.cmake"
	INSTALL_DESTINATION ${INSTALL_CONFIGDIR})

## Install all the helper files
install(FILES	"${CMAKE_CURRENT_BINARY_DIR}/${LIB_CONFIG_NAME}.cmake"
		"${CMAKE_CURRENT_BINARY_DIR}/${LIB_VERSION_NAME}.cmake"
	DESTINATION ${INSTALL_CONFIGDIR})


##########################################
## Install daemon
install(TARGETS TSVDaemon
	RUNTIME DESTINATION ${TSV_DAEMON_DIR})
