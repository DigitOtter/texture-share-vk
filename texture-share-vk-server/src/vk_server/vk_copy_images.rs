use texture_share_vk_base::{ash::vk, vk_cpu_shared_image::VkCpuSharedImage, vk_device::VkDevice};

pub(super) struct VkCopyImages;

impl VkCopyImages {
	pub(super) fn copy_images(
		read_image: (&VkDevice, &VkCpuSharedImage),
		write_images: &[(&VkDevice, &VkCpuSharedImage)],
	) -> Result<(), vk::Result> {
		// let mut wfences = write_images
		// 	.iter()
		// 	.map(|wimg| wimg.0.create_fence(None))
		// 	.collect::<Result<Vec<_>, _>>()?;
		// let rfence = read_image.0.create_fence(None)?;

		// Copy read image into CPU RAM
		read_image.1.cpu_buffer.read_image_to_cpu(
			read_image.0,
			read_image.1.image.image,
			read_image.1.image.image_layout,
			read_image.1.image.get_image_data().width,
			read_image.1.image.get_image_data().height,
		)?;

		// Copy CPU RAM to write images (synchronization should be accomplished with fences)
		write_images
			.iter()
			.map(|wimg| {
				wimg.1.cpu_buffer.write_image_from_cpu(
					wimg.0,
					wimg.1.image.image,
					wimg.1.image.image_layout,
					wimg.1.image.get_image_data().width,
					wimg.1.image.get_image_data().height,
				)
			})
			.collect::<Result<_, _>>()?;

		// TODO: Use fences from this function (allows the write operations to execute in parallel)
		// read_image.0.destroy_fence(rfence);
		// wfences
		// 	.drain(..)
		// 	.zip(write_images.iter())
		// 	.for_each(|w| w.1 .0.destroy_fence(w.0));

		Ok(())
	}
}
