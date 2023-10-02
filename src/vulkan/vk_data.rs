use std::sync::Arc;

use vulkano::{
    command_buffer::{
        allocator::{StandardCommandBufferAllocator, StandardCommandBufferAllocatorCreateInfo},
        AutoCommandBufferBuilder, CommandBufferUsage, PrimaryCommandBufferAbstract,
    },
    device::{
        physical::PhysicalDevice, Device, DeviceCreateInfo, Queue, QueueCreateInfo, QueueFlags,
    },
    instance::{Instance, InstanceCreateInfo, InstanceExtensions},
    memory::allocator::{MemoryAllocator, StandardMemoryAllocator},
    VulkanLibrary,
};

struct VkData {
    instance: Arc<Instance>,
    phys_dev: Arc<PhysicalDevice>,
    dev: Arc<Device>,
    queue: Arc<Queue>,
    queue_index: u32,
    command_buffer: Arc<dyn PrimaryCommandBufferAbstract>,
    memory_allocator: Arc<dyn MemoryAllocator>,
}

impl VkData {
    pub fn new() -> VkData {
        let vk_library = VulkanLibrary::new().expect("No local Vulkan library found");

        let instance_create_info = InstanceCreateInfo::default();
        //instance_create_info.application_name = None;

        let instance = Instance::new(vk_library, instance_create_info)
            .expect("Failed to create Vulkan instance");

        let phys_dev = instance
            .enumerate_physical_devices()
            .expect("Failed to enumerate Vulkan physical devices")
            .find_map(|d| Some(d))
            .unwrap();

        let queue_index: u32 = phys_dev
            .queue_family_properties()
            .iter()
            .enumerate()
            .position(|(_queue_family_index, queue_family_properties)| {
                queue_family_properties
                    .queue_flags
                    .contains(QueueFlags::TRANSFER)
            })
            .expect("Failed to find a Vulkan graphical queue family")
            as _;

        let (dev, mut queues) = Device::new(
            phys_dev.clone(),
            DeviceCreateInfo {
                // here we pass the desired queue family to use by index
                queue_create_infos: vec![QueueCreateInfo {
                    queue_family_index: queue_index,
                    ..Default::default()
                }],
                ..Default::default()
            },
        )
        .expect("Failed to create Vulkan device");

        let queue = queues.next().unwrap();

        let cmd_buf_allocator = StandardCommandBufferAllocator::new(
            dev.clone(),
            StandardCommandBufferAllocatorCreateInfo::default(),
        );
        let cmd_buf_builder = AutoCommandBufferBuilder::primary(
            &cmd_buf_allocator,
            queue_index,
            CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap();

        let command_buffer = cmd_buf_builder.build().unwrap();

        let memory_allocator = StandardMemoryAllocator::new_default(dev.clone());

        VkData {
            instance,
            phys_dev,
            dev,
            queue,
            queue_index,
            command_buffer: Arc::new(command_buffer),
            memory_allocator: Arc::new(memory_allocator),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::VkData;

    #[test]
    fn vk_init() {
        let _ = VkData::new();
    }
}
