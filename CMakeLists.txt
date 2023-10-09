set(PROJECT_NAME "TextureShareVk")
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

set(CMAKE_EXPORT_COMPILE_COMMANDS ON)
set(CMAKE_CXX_STANDARD 20)

find_package(Vulkan REQUIRED)
find_package(OpenGL REQUIRED)

set(CMAKE_POSITION_INDEPENDENT_CODE ON)

set(TSV_RUST_GEN_INCLUDE_DIR
    "${CMAKE_CURRENT_BINARY_DIR}/gen_includes"
    CACHE PATH "Path to directory for generated headers")

set(TSV_INSTALL_BINDIR
    "${CMAKE_INSTALL_SBINDIR}"
    CACHE PATH "Directory to save server executable in")

# ##############################################################################
# Rust Compilation (Use corrosion for compilation and CMake integration)
add_subdirectory("third_party/corrosion")

# Import targets defined in a package or workspace manifest `Cargo.toml` file
corrosion_import_crate(MANIFEST_PATH "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml")

# Targets
add_library(Rust::TextureShareIpc ALIAS texture-share-ipc)
# add_library(Rust::TextureShareVkBase ALIAS texture-share-vk-base)
# add_library(Rust::TextureShareVkServer ALIAS texture-share-vk-server)
add_library(Rust::TextureShareVkClient ALIAS texture-share-vk-client)
add_library(Rust::TextureShareGlClient ALIAS texture-share-gl-client)

corrosion_set_env_vars(texture-share-ipc
                       TSV_RUST_GEN_INCLUDE_DIR=${TSV_RUST_GEN_INCLUDE_DIR})
corrosion_set_env_vars(texture-share-vk-client
                       TSV_RUST_GEN_INCLUDE_DIR=${TSV_RUST_GEN_INCLUDE_DIR})
corrosion_set_env_vars(texture-share-gl-client
                       TSV_RUST_GEN_INCLUDE_DIR=${TSV_RUST_GEN_INCLUDE_DIR})

# ##############################################################################
# Libraries
add_library(TextureShareIpc INTERFACE)
add_library(${PROJECT_NAME}::TextureShareIpc ALIAS TextureShareIpc)

target_include_directories(
  TextureShareIpc
  INTERFACE "$<INSTALL_INTERFACE:${CMAKE_INSTALL_INCLUDEDIR}>"
            "$<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}>"
            "$<BUILD_INTERFACE:${TSV_RUST_TARGET_DIR}>")

target_link_libraries(
  TextureShareIpc INTERFACE $<BUILD_INTERFACE:Rust::TextureShareIpc>
                            $<INSTALL_INTERFACE:texture_share_ipc>)

add_library(TextureShareVkClient INTERFACE)
add_library(${PROJECT_NAME}::TextureShareVkClient ALIAS TextureShareVkClient)

target_include_directories(
  TextureShareIpc
  INTERFACE "$<INSTALL_INTERFACE:${CMAKE_INSTALL_INCLUDEDIR}>"
            "$<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}>"
            "$<BUILD_INTERFACE:${TSV_RUST_TARGET_DIR}>")

target_link_libraries(
  TextureShareVkClient
  INTERFACE $<BUILD_INTERFACE:Rust::TextureShareVkClient>
            $<INSTALL_INTERFACE:texture_share_vk_client> Vulkan::Vulkan)

add_library(TextureShareGlClient INTERFACE)
add_library(${PROJECT_NAME}::TextureShareGlClient ALIAS TextureShareGlClient)

target_include_directories(
  TextureShareGlClient
  INTERFACE "$<INSTALL_INTERFACE:${CMAKE_INSTALL_INCLUDEDIR}>"
            "$<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}>"
            "$<BUILD_INTERFACE:${TSV_RUST_TARGET_DIR}>")

target_link_libraries(
  TextureShareGlClient
  INTERFACE $<BUILD_INTERFACE:Rust::TextureShareGlClient>
            $<INSTALL_INTERFACE:texture_share_gl_client> OpenGL::GL)

# ##############################################################################
# # Install library files
set(INSTALL_CONFIGDIR "${CMAKE_INSTALL_LIBDIR}/cmake/${PROJECT_NAME}")
install(
  TARGETS TextureShareIpc TextureShareVkClient TextureShareGlClient
  EXPORT ${LIB_EXPORT_NAME}
  LIBRARY DESTINATION ${CMAKE_INSTALL_LIBDIR}
  ARCHIVE DESTINATION ${CMAKE_INSTALL_LIBDIR}
  RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR}
  PUBLIC_HEADER DESTINATION ${CMAKE_INSTALL_INCLUDEDIR}/${HEADER_DIRECTORY})

# Manually install library files
install(
  FILES "${CMAKE_CURRENT_BINARY_DIR}/libtexture_share_ipc.so"
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
