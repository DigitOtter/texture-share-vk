# Texture Share Vulkan

Library for exchanging images between running Vulkan and OpenGL processes.

## Usage via CMake

To include the library in a CMake installation:

- Inside your CMakeLists.txt, add `add_subdirectory(<PATH TO texture-share-vk>)`
- For a vulkan based project, add `TextureShareVulkan::TextureShareVulkan` as a library dependency
- For an opengl based project, add `TextureShareVulkan::TextureShareOpenGL` as a library dependency

NOTE: For testing purposes, it will probably be necessary to run the install command at least once so that the daemon executable is installed in the correct location and can be started properly.

## Installation

### Linux

Ensure all dependencies are installed: Vulkan, Boost Interprocess, OpenGL.

```bash
git submodule update --init --recursive
mkdir build
cd build
cmake ..
make
make install
```

Alternatively, the library is also available as a package on the AUR for ArchLinux: `pikaur -S texture-share-vk-git`

## Todos:

[ ] Documentation
[ ] Windows Compatibility
[ ] Add DirectX

