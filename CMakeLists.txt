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

option(ENABLE_TESTING "Compile tests" OFF)

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


set(IPC_MEMORY_LIB_SRC_FILES
	"texture_share_vk/ipc_memory.cpp"
	"texture_share_vk/platform/${PLATFORM_DIRECTORY}/daemon_comm.cpp"
)

set(OPENGL_LIB_SRC_FILES
	"texture_share_vk/opengl/shared_image_gl.cpp"
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

# List executable source files here
set(EXEC_SRC_FILES
	"vk_program/main.cpp"
	"vk_program/vk_test_image_share.cpp"
	"vk_program/vk_program.cpp"
)

# List testing files here
set(TEST_SRC_FILES
)


##########################################
## Configure header
set(TSV_DAEMON_NAME "SharedImagesDaemon")
if(NOT WIN32)
	set(TSV_DAEMON_DIR "${CMAKE_INSTALL_FULL_BINDIR}")
	set(TSV_DAEMON_SOCKET_DIR "${CMAKE_INSTALL_FULL_RUNSTATEDIR}/${TSV_DAEMON_NAME}" CACHE PATH "Path to daemon socket directory")
	set(TSV_DAEMON_LOCK_FILE  "${CMAKE_INSTALL_FULL_RUNSTATEDIR}/${TSV_DAEMON_NAME}/${TSV_DAEMON_NAME}.lock" CACHE FILEPATH "Daemon lock file")
else()
	if(SIZEOF_VOID_P_CODE IS 4)
		set(TSV_DAEMON_DIR "${CMAKE_INSTALL_PREFIX}/bin/32bit")
	else()
		set(TSV_DAEMON_DIR "${CMAKE_INSTALL_PREFIX}/bin/64bit")
	endif()
endif()

configure_file("texture_share_vk/platform/config.h.in" "${CMAKE_BINARY_DIR}/include/texture_share_vk/platform/config.h" @ONLY)


##########################################
## IPC Memory Library
add_library(IPCMemory SHARED ${IPC_MEMORY_LIB_SRC_FILES})
add_library("${NAMESPACE_NAME}::IPCMemory" ALIAS IPCMemory)
target_compile_options(IPCMemory PUBLIC $<$<OR:$<CXX_COMPILER_ID:Clang>,$<CXX_COMPILER_ID:GNU>>:-Wall -Wextra>)

target_include_directories(IPCMemory
	PUBLIC
		"$<INSTALL_INTERFACE:${CMAKE_INSTALL_INCLUDEDIR}>"
		"$<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}>"
		"$<BUILD_INTERFACE:${CMAKE_BINARY_DIR}/include>"

	PRIVATE
)

target_link_libraries(IPCMemory
	PUBLIC
		Boost::boost
		Boost::date_time

	PRIVATE
)


##########################################
## Vulkan Library
add_library("${LIBRARY_NAME}" SHARED ${LIB_SRC_FILES})
add_library("${NAMESPACE_NAME}::${LIBRARY_NAME}" ALIAS ${LIBRARY_NAME})
target_compile_options(${LIBRARY_NAME} PUBLIC $<$<OR:$<CXX_COMPILER_ID:Clang>,$<CXX_COMPILER_ID:GNU>>:-Wall -Wextra>)

target_include_directories(${LIBRARY_NAME}
	PUBLIC
		"$<INSTALL_INTERFACE:${CMAKE_INSTALL_INCLUDEDIR}>"
		"$<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}>"
		"$<BUILD_INTERFACE:${CMAKE_BINARY_DIR}/include>"

	PRIVATE
)

target_link_libraries(${LIBRARY_NAME}
	PUBLIC
		Vulkan::Vulkan
		Boost::boost
		Boost::date_time

		vk-bootstrap::vk-bootstrap

		"${NAMESPACE_NAME}::IPCMemory"

	PRIVATE
		#volk::volk
)

##########################################
## OpenGL Library
add_library(TextureShareOpenGL SHARED ${OPENGL_LIB_SRC_FILES})
add_library("${NAMESPACE_NAME}::TextureShareOpenGL" ALIAS TextureShareOpenGL)
target_compile_options(TextureShareOpenGL PUBLIC $<$<OR:$<CXX_COMPILER_ID:Clang>,$<CXX_COMPILER_ID:GNU>>:-Wall -Wextra>)


target_include_directories(TextureShareOpenGL
	PUBLIC
		"$<INSTALL_INTERFACE:${CMAKE_INSTALL_INCLUDEDIR}>"
		"$<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}>"
		"$<BUILD_INTERFACE:${CMAKE_BINARY_DIR}/include>"

	PRIVATE
)

target_link_libraries(TextureShareOpenGL
	PUBLIC
		OpenGL::OpenGL

	PRIVATE
)

##########################################
## Shared Images Daemon
add_executable(${TSV_DAEMON_NAME} ${VK_D_SRC_FILES})
target_compile_options(${TSV_DAEMON_NAME} PUBLIC $<$<OR:$<CXX_COMPILER_ID:Clang>,$<CXX_COMPILER_ID:GNU>>:-Wall -Wextra>)

target_link_libraries(${TSV_DAEMON_NAME}
	PUBLIC
		"${NAMESPACE_NAME}::${LIBRARY_NAME}"
		"${NAMESPACE_NAME}::IPCMemory"
)


##########################################
## Executable
if(NOT "${EXEC_SRC_FILES}" STREQUAL "")
	find_package(SDL2 REQUIRED)

	add_executable("${EXECUTABLE_NAME}" ${EXEC_SRC_FILES})
	target_link_libraries("${EXECUTABLE_NAME}"
		${LIBRARY_NAME}
		SDL2::SDL2
		#volk::volk
	)
endif()


##########################################
## Tests
if(${ENABLE_TESTING} AND NOT "${TEST_SRC_FILES}" STREQUAL "")
	enable_testing()

	add_executable(${TEST_NAME} ${TEST_SRC_FILES})
	target_link_libraries(${TEST_NAME}
		PUBLIC
	)

	gtest_discover_tests(${TEST_NAME}
		WORKING_DIRECTORY "${PROJECT_SOURCE_DIR}"
		EXTRA_ARGS -VV)
endif()


##########################################
## Install library files
set(INSTALL_CONFIGDIR "${CMAKE_INSTALL_LIBDIR}/cmake/${PROJECT_NAME}")
install(TARGETS ${LIBRARY_NAME} IPCMemory
	EXPORT ${LIB_EXPORT_NAME}
	LIBRARY DESTINATION ${CMAKE_INSTALL_LIBDIR}
	ARCHIVE DESTINATION ${CMAKE_INSTALL_LIBDIR}
	RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR}

	PUBLIC_HEADER DESTINATION ${CMAKE_INSTALL_INCLUDEDIR}/${HEADER_DIRECTORY})

# Install headers
install(DIRECTORY ${HEADER_DIRECTORY} "${CMAKE_BINARY_DIR}/include/${HEADER_DIRECTORY}"
	DESTINATION "${CMAKE_INSTALL_INCLUDEDIR}/${HEADER_DIRECTORY}"
	FILES_MATCHING
		PATTERN "*.h"
		PATTERN "*.hpp")

# Export target
install(EXPORT ${LIB_EXPORT_NAME}
	DESTINATION ${INSTALL_CONFIGDIR}
	FILE "${LIB_EXPORT_NAME}.cmake"
	NAMESPACE "${NAMESPACE_NAME}::")

install(EXPORT vk-bootstrap-targets
	DESTINATION "${INSTALL_CONFIGDIR}/vk-bootstrap")

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
## Install executable
install(TARGETS ${TSV_DAEMON_NAME}
	RUNTIME DESTINATION ${TSV_DAEMON_DIR})

if(TARGET ${EXECUTABLE_NAME})
	install(TARGETS ${EXECUTABLE_NAME}
		RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR})
endif()

