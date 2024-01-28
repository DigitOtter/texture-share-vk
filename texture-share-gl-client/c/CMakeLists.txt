set(PROJECT_NAME "GlExtern")
set(HEADER_DIRECTORY "gl_extern")

set(NAMESPACE_NAME "${PROJECT_NAME}")

set(LIBRARY_NAME "${PROJECT_NAME}")
set(EXECUTABLE_NAME "${PROJECT_NAME}Exec")
set(TEST_NAME "${PROJECT_NAME}Tests")

set(LIB_EXPORT_NAME "${LIBRARY_NAME}Targets")
set(LIB_CONFIG_NAME "${LIBRARY_NAME}Config")
set(LIB_VERSION_NAME "${LIB_CONFIG_NAME}Version")

option(CMAKE_EXPORT_COMPILE_COMMANDS "Export compile commands" ON)
option(BUILD_SHARED_LIBS OFF)

cmake_minimum_required(VERSION 3.18)
project("${PROJECT_NAME}" VERSION 1.0.0)

include(GNUInstallDirs)

set(GLAD_API
    ""
    CACHE STRING "")
set(GLAD_GENERATOR
    "c"
    CACHE STRING "")
set(GLAD_EXTENSIONS
    "GL_EXT_memory_object,GL_EXT_memory_object_fd,GL_EXT_semaphore,GL_EXT_semaphore_fd,GL_EXT_texture_storage"
    CACHE STRING "")
set(GLAD_SPEC
    "gl"
    CACHE STRING "")
set(GLAD_INSTALL
    ON
    CACHE BOOL "")
set(GLAD_EXPORT
    ON
    CACHE BOOL "")
add_subdirectory("third_party/glad")

find_package(OpenGL REQUIRED)

set(LIB_SRC_FILES "gl_extern/gl_extern.c")

# ##############################################################################
# Library
add_library("${LIBRARY_NAME}" ${LIB_SRC_FILES})
add_library("${NAMESPACE_NAME}::${LIBRARY_NAME}" ALIAS ${LIBRARY_NAME})
target_compile_options(
    ${LIBRARY_NAME}
    PUBLIC $<$<OR:$<C_COMPILER_ID:Clang>,$<C_COMPILER_ID:GNU>>:-Wall -Wextra>)

target_include_directories(
    ${LIBRARY_NAME}
    PUBLIC "$<INSTALL_INTERFACE:${CMAKE_INSTALL_INCLUDEDIR}>"
           "$<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}>"
           "$<BUILD_INTERFACE:${GLAD_INCLUDE_DIRS}>"
    PRIVATE)

target_link_libraries(
    ${LIBRARY_NAME}
    PUBLIC
    PRIVATE glad
            # OpenGL::GL
)

# ##############################################################################
# Install library files
install(
    TARGETS ${LIBRARY_NAME}
    EXPORT ${LIB_EXPORT_NAME}
    LIBRARY DESTINATION ${CMAKE_INSTALL_LIBDIR}
    ARCHIVE DESTINATION ${CMAKE_INSTALL_LIBDIR}
    RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR}
    PUBLIC_HEADER DESTINATION ${CMAKE_INSTALL_INCLUDEDIR}/${HEADER_DIRECTORY})

# Install headers
install(
    DIRECTORY ${HEADER_DIRECTORY} ${GLAD_INCLUDE_DIRS}/
    DESTINATION "${CMAKE_INSTALL_INCLUDEDIR}"
    FILES_MATCHING
    PATTERN "*.h"
    PATTERN "*.hpp")
