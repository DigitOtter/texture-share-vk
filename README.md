# Texture Sharing between Vulkan and OpenGL instances

An API to share GPU texture memory between processes. Can be used to exchange images without performing a CPU roundtrip. 

## Build

### Linux

- Download repository
- Execute inside the repository directory: 
  ```bash
  git submodule update --init --recursive
  cmake -S . -B build -DCMAKE_BUILD_TYPE=Release -GNinja
  cmake --build build
  sudo cmake --install build
  ```

### Windows

- Currently not supported (I'd recommend using Spout2 on Windows)

## Installation

### Linux

#### Arch Linux

Available via the `texture-share-vk-git` AUR package:

```bash
pikaur -S texture-share-vk-git
```

## Todos

- [ ] Documentation
- [ ] Fallback to sharing RAM memory if GPU does not support texture sharing or if Vulkan/OpenGL instances are running on different GPUs
- [ ] Maybe Windows version


