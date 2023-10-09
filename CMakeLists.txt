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

# ##############################################################################
# Rust Compilation (Use corrosion for compilation and CMake integration)
add_subdirectory("third_party/corrosion")

# Import targets defined in a package or workspace manifest `Cargo.toml` file
corrosion_import_crate(MANIFEST_PATH "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml")

# Targets
add_library(Rust::TextureShareIpc ALIAS texture-share-ipc)

set(TSV_RUST_TARGET_DIR
    "${CMAKE_CURRENT_SOURCE_DIR}/target"
    CACHE PATH "Path to Rust compile directory")

# ##############################################################################
# Libraries
add_library(TextureShareIpc INTERFACE)
add_library(${PROJECT_NAME}::TextureShareIpc ALIAS TextureShareIpc)

target_include_directories(
  TextureShareIpc
  INTERFACE "$<INSTALL_INTERFACE:${CMAKE_INSTALL_INCLUDEDIR}>"
            "$<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}>"
            "$<BUILD_INTERFACE:${TSV_RUST_TARGET_DIR}>")

target_link_libraries(TextureShareIpc INTERFACE Rust::TextureShareIpc)

# #
# ##############################################################################
# # Install library files set(INSTALL_CONFIGDIR
# "${CMAKE_INSTALL_LIBDIR}/cmake/${PROJECT_NAME}") install( TARGETS
# ${LIBRARY_NAME} EXPORT ${LIB_EXPORT_NAME} LIBRARY DESTINATION
# ${CMAKE_INSTALL_LIBDIR} ARCHIVE DESTINATION ${CMAKE_INSTALL_LIBDIR} RUNTIME
# DESTINATION ${CMAKE_INSTALL_BINDIR} PUBLIC_HEADER DESTINATION
# ${CMAKE_INSTALL_INCLUDEDIR}/${HEADER_DIRECTORY} )

# # Install headers install( DIRECTORY ${HEADER_DIRECTORY} DESTINATION
# "${CMAKE_INSTALL_INCLUDEDIR}/${HEADER_DIRECTORY}" FILES_MATCHING PATTERN "*.h"
# PATTERN "*.hpp" )

# # Export target install( EXPORT ${LIB_EXPORT_NAME} DESTINATION
# ${INSTALL_CONFIGDIR} FILE "${LIB_EXPORT_NAME}.cmake" NAMESPACE
# "${NAMESPACE_NAME}::" )

# # create version and config files include(CMakePackageConfigHelpers)
# write_basic_package_version_file(
# "${CMAKE_CURRENT_BINARY_DIR}/${LIB_VERSION_NAME}.cmake" VERSION
# ${PROJECT_VERSION} COMPATIBILITY AnyNewerVersion )

# configure_package_config_file(
# "${CMAKE_CURRENT_LIST_DIR}/cmake/ProjectConfig.cmake.in"
# "${CMAKE_CURRENT_BINARY_DIR}/${LIB_CONFIG_NAME}.cmake" INSTALL_DESTINATION
# ${INSTALL_CONFIGDIR} )

# # Install all the helper files install(FILES
# "${CMAKE_CURRENT_BINARY_DIR}/${LIB_CONFIG_NAME}.cmake"
# "${CMAKE_CURRENT_BINARY_DIR}/${LIB_VERSION_NAME}.cmake" DESTINATION
# ${INSTALL_CONFIGDIR})
