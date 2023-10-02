use std::{alloc::Layout, sync::Arc};

use vulkano::{
    command_buffer::{
        allocator::CommandBufferAllocator, AutoCommandBufferBuilder, BlitImageInfo,
        CommandBufferUsage, CopyImageInfo, PrimaryCommandBufferAbstract,
    },
    device::Device,
    format::Format,
    image::{
        sys::{Image, ImageCreateInfo, RawImage},
        ImageCreateFlags, ImageDimensions, ImageError, ImageLayout, ImageUsage, StorageImage,
    },
    memory::{
        allocator::{
            AllocationCreateInfo, AllocationType, MemoryAllocatePreference, MemoryAllocator,
        },
        DedicatedAllocation, ExternalMemoryHandleTypes,
    },
    sync::MemoryBarrier,
};

struct VkSharedImage {
    image: Image,
    layout: ImageLayout,
}

impl VkSharedImage {
    fn new(
        vk_device: Arc<Device>,
        width: u32,
        height: u32,
        format: Format,
        memory_alloc: Arc<dyn MemoryAllocator>,
    ) -> Result<VkSharedImage, ImageError> {
        let layout = ImageLayout::Undefined;

        let mut image_create_info = ImageCreateInfo::default();
        image_create_info.dimensions = ImageDimensions::Dim2d {
            width,
            height,
            array_layers: 1,
        };
        image_create_info.external_memory_handle_types = ExternalMemoryHandleTypes::OPAQUE_FD;
        image_create_info.initial_layout = layout;
        image_create_info.mip_levels = 1;
        image_create_info.usage =
            ImageUsage::COLOR_ATTACHMENT | ImageUsage::TRANSFER_DST | ImageUsage::TRANSFER_SRC;
        image_create_info.format = Some(format);

        let image = RawImage::new(vk_device, image_create_info)?;
        let alloc = memory_alloc.allocate(
            *image.memory_requirements().first().unwrap(),
            AllocationType::Linear,
            AllocationCreateInfo {
                allocate_preference: MemoryAllocatePreference::AlwaysAllocate,
                ..Default::default()
            },
            Some(DedicatedAllocation::Image(&image)),
        );
        let image = image.bind_memory(alloc).map_err(|e| e.0)?;

        Ok(VkSharedImage { image, layout })
    }

    fn send_image<A: CommandBufferAllocator>(
        &self,
        send_image: &Image,
        send_image_layout: Layout,
        cmd_buf_allocator: &A,
        queue_index: u32,
    ) {
        let cb = AutoCommandBufferBuilder::primary(
            cmd_buf_allocator,
            queue_index,
            CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap()
        .blit_image(blit_image_info);
    }
}
