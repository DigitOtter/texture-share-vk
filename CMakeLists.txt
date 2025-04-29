set(Rust_TOOLCHAIN "nightly" CACHE STRING "Rust Toolchain")

set(PROJECT_NAME "TextureShareVk")
set(HEADER_DIRECTORY "texture_share_vk")

set(NAMESPACE_NAME "${PROJECT_NAME}")

set(LIBRARY_NAME "${PROJECT_NAME}")
set(EXECUTABLE_NAME "${PROJECT_NAME}Exec")
set(TEST_NAME "${PROJECT_NAME}Tests")

set(LIB_EXPORT_NAME "${LIBRARY_NAME}Targets")
set(LIB_CONFIG_NAME "${LIBRARY_NAME}Config")
set(LIB_VERSION_NAME "${LIB_CONFIG_NAME}Version")

cmake_minimum_required(VERSION 3.27)
project("${PROJECT_NAME}" VERSION 1.0.0)

include(GNUInstallDirs)

set(CMAKE_EXPORT_COMPILE_COMMANDS ON)
set(CMAKE_CXX_STANDARD 20)

find_package(Vulkan REQUIRED)
find_package(OpenGL REQUIRED)

set(CMAKE_POSITION_INDEPENDENT_CODE ON)
set(CMAKE_INTERPROCEDURAL_OPTIMIZATION
    ON
    CACHE BOOL "Activate LTO")

set(TSV_RUST_GEN_INCLUDE_DIR
    "${CMAKE_CURRENT_BINARY_DIR}/gen_includes"
    CACHE PATH "Path to directory for generated headers")

set(TSV_INSTALL_BINDIR
    "${CMAKE_INSTALL_BINDIR}"
    CACHE PATH "Directory to save server executable in")

option(TSV_ABSOLUTE_PATH "Library stores absolute path to tsv executable" ON)

set(TSV_VK_SERVER_DEFAULT_LOCKFILE_PATH
    "/tmp/vk_server/vk_server.lock"
    CACHE PATH "Default VkServr lockfile path")
set(TSV_VK_SERVER_DEFAULT_SOCKET_PATH
    "/tmp/vk_server/vk_server.sock"
    CACHE PATH "Default VkServr socket path")
set(TSV_VK_SERVER_DEFAULT_SHMEM_PREFIX
    "shmem_img_"
    CACHE STRING "Default VkServr shared memory prefix")

set(DOWNLOAD_GLAD_SPECS ON CACHE BOOL "Download gl specs")
if(DOWNLOAD_GLAD_SPECS)
    set(DGS "ON")
else()
    set(DGS "OFF")
endif()

# ##############################################################################
# Include Rust internal cmake targets (Only to create compile_commands.json)
# add_subdirectory("texture-share-vk-base/cpp" EXCLUDE_FROM_ALL)
# add_subdirectory("texture-share-gl-client/cpp" EXCLUDE_FROM_ALL)

# ##############################################################################
# Rust Compilation (Use corrosion for compilation and CMake integration)
add_subdirectory("third_party/corrosion")

# Import targets defined in a package or workspace manifest `Cargo.toml` file
corrosion_import_crate(MANIFEST_PATH "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml")

add_dependencies(cargo-prebuild_texture-share-vk-server
                 cargo-build_texture_share_vk_base)
add_dependencies(cargo-prebuild_texture_share_vk_client
                 cargo-build_texture_share_vk_base)

# Rust build targets
add_library(Rust::TextureShareIpc ALIAS texture_share_ipc)
add_library(Rust::TextureShareVkBase ALIAS texture_share_vk_base)
# add_library(Rust::TextureShareVkServer ALIAS texture_share_vk_server)
add_library(Rust::TextureShareVkClient ALIAS texture_share_vk_client)
add_library(Rust::TextureShareGlClient ALIAS texture_share_gl_client)

corrosion_set_env_vars(texture_share_ipc
                       TSV_RUST_GEN_INCLUDE_DIR=${TSV_RUST_GEN_INCLUDE_DIR})
corrosion_set_env_vars(texture-share-vk-server
                       TSV_RUST_GEN_INCLUDE_DIR=${TSV_RUST_GEN_INCLUDE_DIR})
corrosion_set_env_vars(texture_share_vk_client
                       TSV_RUST_GEN_INCLUDE_DIR=${TSV_RUST_GEN_INCLUDE_DIR})
corrosion_set_env_vars(texture_share_gl_client
                       TSV_RUST_GEN_INCLUDE_DIR=${TSV_RUST_GEN_INCLUDE_DIR}
                       DOWNLOAD_GLAD_SPECS=${DGS})

# ##############################################################################
# Libraries

# Ipc
add_library(TextureShareIpc INTERFACE)
add_library(${PROJECT_NAME}::TextureShareIpc ALIAS TextureShareIpc)

target_include_directories(
  TextureShareIpc INTERFACE "$<INSTALL_INTERFACE:${CMAKE_INSTALL_INCLUDEDIR}>"
                            "$<BUILD_INTERFACE:${TSV_RUST_GEN_INCLUDE_DIR}>")

target_link_libraries(
  TextureShareIpc INTERFACE $<BUILD_INTERFACE:Rust::TextureShareIpc>
                            $<INSTALL_INTERFACE:texture_share_ipc.so>)

# VkClient
add_library(TextureShareVkClient INTERFACE)
add_library(${PROJECT_NAME}::TextureShareVkClient ALIAS TextureShareVkClient)

target_include_directories(
  TextureShareVkClient
  INTERFACE "$<INSTALL_INTERFACE:${CMAKE_INSTALL_INCLUDEDIR}>"
            "$<BUILD_INTERFACE:${TSV_RUST_GEN_INCLUDE_DIR}>")

target_link_libraries(
  TextureShareVkClient
  INTERFACE $<BUILD_INTERFACE:Rust::TextureShareVkClient>
            # $<BUILD_INTERFACE:Vulkan::Vulkan>
            $<INSTALL_INTERFACE:libtexture_share_vk_client.so>)

# GlClient
add_library(TextureShareGlClient INTERFACE)
add_library(${PROJECT_NAME}::TextureShareGlClient ALIAS TextureShareGlClient)

target_include_directories(
  TextureShareGlClient
  INTERFACE "$<INSTALL_INTERFACE:${CMAKE_INSTALL_INCLUDEDIR}>"
            "$<BUILD_INTERFACE:${TSV_RUST_GEN_INCLUDE_DIR}>")

target_link_libraries(
  TextureShareGlClient
  INTERFACE $<BUILD_INTERFACE:Rust::TextureShareGlClient>
            # $<BUILD_INTERFACE:OpenGL::OpenGL>
            $<INSTALL_INTERFACE:libtexture_share_gl_client.so>)

# #
# ##############################################################################
# Cpp Wrappers
set(VK_SERVER_EXECUTABLE "texture-share-vk-server")
if(TSV_ABSOLUTE_PATH)
  set(VK_SERVER_EXECUTABLE
      "${CMAKE_INSTALL_PREFIX}/${TSV_INSTALL_BINDIR}/${VK_SERVER_EXECUTABLE}")
endif()
configure_file("${CMAKE_CURRENT_SOURCE_DIR}/cpp/texture_share_vk/config.h.in"
               "${TSV_RUST_GEN_INCLUDE_DIR}/texture_share_vk/config.h" @ONLY)

# VkSetup wrapper
add_library(TextureShareVkSetupCpp SHARED
            "cpp/texture_share_vk/texture_share_vk_setup.cpp")
add_library(${PROJECT_NAME}::TextureShareVkSetupCpp ALIAS
            TextureShareVkSetupCpp)

add_dependencies(TextureShareVkSetupCpp cargo-build_texture_share_vk_client)

target_include_directories(
  TextureShareVkSetupCpp
  PUBLIC "$<INSTALL_INTERFACE:${CMAKE_INSTALL_INCLUDEDIR}>"
         "$<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/cpp>"
         "$<BUILD_INTERFACE:${TSV_RUST_GEN_INCLUDE_DIR}>")

# VkClient wrapper
add_library(TextureShareVkClientCpp SHARED
            "cpp/texture_share_vk/texture_share_vk_client.cpp")
add_library(${PROJECT_NAME}::TextureShareVkClientCpp ALIAS
            TextureShareVkClientCpp)

target_link_libraries(
  TextureShareVkClientCpp PUBLIC ${PROJECT_NAME}::TextureShareVkClient
                                 ${PROJECT_NAME}::TextureShareVkSetupCpp)

# GlClient wrapper
add_library(TextureShareGlClientCpp SHARED
            "cpp/texture_share_gl/texture_share_gl_client.cpp")
add_library(${PROJECT_NAME}::TextureShareGlClientCpp ALIAS
            TextureShareGlClientCpp)

target_link_libraries(TextureShareGlClientCpp
                      PUBLIC ${PROJECT_NAME}::TextureShareGlClient)

# Lazy Test Linker. Remove later
add_executable(test_gl_client "cpp/test_main.cpp")
target_link_libraries(test_gl_client ${PROJECT_NAME}::TextureShareGlClientCpp
                      OpenGL::GL)
add_executable(test_vk_client "cpp/test_main.cpp")
target_link_libraries(test_vk_client ${PROJECT_NAME}::TextureShareVkClientCpp
                      Vulkan::Vulkan)

# ##############################################################################
# Install library files
set(INSTALL_CONFIGDIR "${CMAKE_INSTALL_LIBDIR}/cmake/${PROJECT_NAME}")
install(
  TARGETS TextureShareIpc TextureShareVkClient TextureShareGlClient
          TextureShareGlClientCpp TextureShareVkClientCpp TextureShareVkSetupCpp
  EXPORT ${LIB_EXPORT_NAME}
  LIBRARY DESTINATION ${CMAKE_INSTALL_LIBDIR}
  ARCHIVE DESTINATION ${CMAKE_INSTALL_LIBDIR}
  RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR}
  PUBLIC_HEADER DESTINATION ${CMAKE_INSTALL_INCLUDEDIR})

# Manually install library files
install(
  PROGRAMS "${CMAKE_CURRENT_BINARY_DIR}/libtexture_share_ipc.so"
           "${CMAKE_CURRENT_BINARY_DIR}/libtexture_share_vk_client.so"
           "${CMAKE_CURRENT_BINARY_DIR}/libtexture_share_gl_client.so" TYPE LIB)

# Manually install executables
install(PROGRAMS "${CMAKE_CURRENT_BINARY_DIR}/texture-share-vk-server"
        DESTINATION "${TSV_INSTALL_BINDIR}")

# Install headers
install(
  DIRECTORY "${TSV_RUST_GEN_INCLUDE_DIR}/"
  DESTINATION "${CMAKE_INSTALL_INCLUDEDIR}"
  FILES_MATCHING
  PATTERN "*.h"
  PATTERN "*.hpp")

install(
  DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}/cpp/"
  DESTINATION "${CMAKE_INSTALL_INCLUDEDIR}"
  FILES_MATCHING
  PATTERN "*.h"
  PATTERN "*.hpp")

# Export target
install(
  EXPORT ${LIB_EXPORT_NAME}
  DESTINATION ${INSTALL_CONFIGDIR}
  FILE "${LIB_EXPORT_NAME}.cmake"
  NAMESPACE "${NAMESPACE_NAME}::")

# create version and config files
include(CMakePackageConfigHelpers)
write_basic_package_version_file(
  "${CMAKE_CURRENT_BINARY_DIR}/${LIB_VERSION_NAME}.cmake"
  VERSION ${PROJECT_VERSION}
  COMPATIBILITY AnyNewerVersion)

configure_package_config_file(
  "${CMAKE_CURRENT_LIST_DIR}/cmake/ProjectConfig.cmake.in"
  "${CMAKE_CURRENT_BINARY_DIR}/${LIB_CONFIG_NAME}.cmake"
  INSTALL_DESTINATION ${INSTALL_CONFIGDIR})

# Install all the helper files
install(FILES "${CMAKE_CURRENT_BINARY_DIR}/${LIB_CONFIG_NAME}.cmake"
              "${CMAKE_CURRENT_BINARY_DIR}/${LIB_VERSION_NAME}.cmake"
        DESTINATION ${INSTALL_CONFIGDIR})
