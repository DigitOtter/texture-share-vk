use cxx::UniquePtr;
use raw_sync::locks::ReadLockGuard;
use raw_sync::Timeout;
use std::collections::HashMap;
use std::ffi::CStr;
use std::io::{Error, ErrorKind};
use std::{
    mem::ManuallyDrop,
    os::fd::{IntoRawFd, OwnedFd},
    time::Duration,
};

use crate::platform::img_data::ImgName;
use crate::platform::linux::ipc_commands::CommFindImage;
use crate::platform::linux::ipc_shmem::ShmemDataInternal;
use crate::vulkan::vk_setup::VkFence;
use crate::vulkan::vk_shared_image::ffi::{VkImageLayout, VkOffset3D};
use crate::vulkan::vk_shared_image::VkImage;
use crate::{
    platform::{
        img_data::{self, ImgData, ImgFormat},
        linux::{
            ipc_commands::{CommInitImage, CommandData, CommandMsg, CommandTag, ResultInitImage},
            ipc_shmem::IpcShmem,
            ipc_unix_socket::IpcConnection,
        },
    },
    vulkan::{
        vk_setup::ffi::VkSetup,
        vk_shared_image::{
            self,
            ffi::{
                vk_share_handles_from_fd, vk_shared_image_new, ShareHandles, SharedImageData,
                VkFormat, VkSharedImage,
            },
        },
    },
};

use super::server::ImageData;

pub struct Client {
    connection: IpcConnection,
    vk_setup: UniquePtr<VkSetup>,
    shared_images: HashMap<String, ImageData>,
    timeout: Duration,
}

impl Drop for Client {
    fn drop(&mut self) {
        // Ensure that images are cleared before destroying vulkan instance
        self.shared_images.clear();
    }
}

impl Client {
    const IPC_TIMEOUT: Duration = Duration::from_millis(5000);

    pub fn new(
        socket_path: &str,
        vk_setup: UniquePtr<VkSetup>,
        timeout: Duration,
    ) -> Result<Client, Error> {
        let connection = IpcConnection::try_connect(socket_path, timeout)?;
        if connection.is_none() {
            return Err(Error::new(
                ErrorKind::Interrupted,
                format!("Connection to '{}' timed out", socket_path),
            ));
        }

        let shared_images = HashMap::default();

        Ok(Client {
            connection: connection.unwrap(),
            vk_setup,
            shared_images,
            timeout,
        })
    }

    pub fn get_vk_setup(&self) -> &UniquePtr<VkSetup> {
        &self.vk_setup
    }

    pub fn get_vk_setup_mut(&mut self) -> &mut UniquePtr<VkSetup> {
        &mut self.vk_setup
    }

    pub fn init_image(
        &mut self,
        image_name: &str,
        width: u32,
        height: u32,
        format: ImgFormat,
        overwrite_existing: bool,
    ) -> Result<Option<()>, Box<dyn std::error::Error>> {
        let image_name_buf = ImgData::convert_shmem_str_to_array(image_name);
        let cmd_msg = CommandMsg {
            tag: CommandTag::InitImage,
            data: CommandData {
                init_img: ManuallyDrop::new(CommInitImage {
                    image_name: image_name_buf,
                    shmem_name: image_name_buf,
                    width,
                    height,
                    format,
                    overwrite_existing,
                }),
            },
        };

        self.connection.send_command(cmd_msg)?;

        // Receive message and check for validity
        let res_msg = self.connection.recv_result()?;
        let res_data: Option<&ImgData> = match &res_msg {
            None => Ok(None),
            Some(msg) => match msg.tag {
                CommandTag::InitImage => {
                    let data = unsafe { &msg.data.init_img };
                    if data.image_created {
                        Ok(Some(&data.img_data))
                    } else {
                        Ok(None)
                    }
                }
                _ => Err(Box::new(Error::new(
                    ErrorKind::InvalidData,
                    "Received invalid data from server",
                ))),
            },
        }?;

        // Don't import image if not created
        if res_data.is_none() {
            return Ok(None);
        }

        let res_data = res_data.unwrap();

        let mut share_handles = self.connection.recv_ancillary(1)?;

        self.connection.send_ack()?;

        let res = self.add_new_image(&res_data, &mut share_handles)?;

        let res = match res {
            Some(_) => Some(()),
            None => None,
        };
        Ok(res)
    }

    pub fn find_image(
        &mut self,
        image_name: &str,
        force_update: bool,
    ) -> Result<Option<()>, Box<dyn std::error::Error>> {
        let res = self.find_image_internal(image_name, force_update)?;
        let res = match res {
            Some(_) => Some(()),
            None => None,
        };
        Ok(res)
    }

    pub fn find_image_data(
        &mut self,
        image_name: &str,
        force_update: bool,
    ) -> Result<Option<(ReadLockGuard, &ShmemDataInternal)>, Box<dyn std::error::Error>> {
        let res = self.find_image_internal(image_name, force_update)?;
        let res = match res {
            Some(image_data) => {
                let rlock: ReadLockGuard = image_data
                    .ipc_info
                    .acquire_rlock(Timeout::Val(Client::IPC_TIMEOUT))?;
                let rdata = IpcShmem::acquire_rdata(&rlock);
                Some((rlock, rdata))
            }
            None => None,
        };
        Ok(res)
    }

    pub fn send_image(
        &mut self,
        image_name: &str,
        image: VkImage,
        layout: VkImageLayout,
        fence: VkFence,
    ) -> Result<Option<()>, Box<dyn std::error::Error>> {
        let local_image = self.shared_images.get_mut(image_name);
        if local_image.is_none() {
            return Ok(None);
        }

        let local_image = local_image.unwrap();
        local_image
            .vk_shared_image
            .as_mut()
            .unwrap()
            .recv_image_blit(
                self.vk_setup.get_vk_queue(),
                self.vk_setup.get_vk_command_buffer(),
                image,
                layout,
                fence,
            );

        Ok(Some(()))
    }

    pub fn send_image_with_extents(
        &mut self,
        image_name: &str,
        image: VkImage,
        layout: VkImageLayout,
        fence: VkFence,
        extents: &[VkOffset3D; 2],
    ) -> Result<Option<()>, Box<dyn std::error::Error>> {
        let local_image = self.shared_images.get_mut(image_name);
        if local_image.is_none() {
            return Ok(None);
        }
        let local_image = local_image.unwrap();
        unsafe {
            local_image
                .vk_shared_image
                .as_mut()
                .unwrap()
                .recv_image_blit_with_extents(
                    self.vk_setup.get_vk_queue(),
                    self.vk_setup.get_vk_command_buffer(),
                    image,
                    layout,
                    fence,
                    extents.as_ptr(),
                );
        }
        Ok(Some(()))
    }

    pub fn recv_image(
        &mut self,
        image_name: &str,
        image: VkImage,
        layout: VkImageLayout,
        fence: VkFence,
    ) -> Result<Option<()>, Box<dyn std::error::Error>> {
        let local_image = self.shared_images.get_mut(image_name);
        if local_image.is_none() {
            return Ok(None);
        }

        let local_image = local_image.unwrap();
        local_image
            .vk_shared_image
            .as_mut()
            .unwrap()
            .send_image_blit(
                self.vk_setup.get_vk_queue(),
                self.vk_setup.get_vk_command_buffer(),
                image,
                layout,
                fence,
            );

        Ok(Some(()))
    }

    pub fn recv_image_with_extents(
        &mut self,
        image_name: &str,
        image: VkImage,
        layout: VkImageLayout,
        fence: VkFence,
        extents: &[VkOffset3D; 2],
    ) -> Result<Option<()>, Box<dyn std::error::Error>> {
        let local_image = self.shared_images.get_mut(image_name);
        if local_image.is_none() {
            return Ok(None);
        }
        let local_image = local_image.unwrap();
        unsafe {
            local_image
                .vk_shared_image
                .as_mut()
                .unwrap()
                .send_image_blit_with_extents(
                    self.vk_setup.get_vk_queue(),
                    self.vk_setup.get_vk_command_buffer(),
                    image,
                    layout,
                    fence,
                    extents.as_ptr(),
                );
        }
        Ok(Some(()))
    }

    fn add_new_image(
        &mut self,
        img_data: &ImgData,
        share_handles: &mut Vec<OwnedFd>,
    ) -> Result<Option<&ImageData>, Box<dyn std::error::Error>> {
        // TODO: Update if sharing more handles
        debug_assert_eq!(share_handles.len(), 1);
        let fd = share_handles.pop().unwrap().into_raw_fd();
        let share_handles = vk_share_handles_from_fd(fd);

        let image_name = ImgData::convert_shmem_array_to_str(&img_data.name);
        let image_data = self.create_local_image(img_data, share_handles)?;
        self.shared_images
            .insert(image_name.to_string(), image_data);

        Ok(Some(self.shared_images.get(&image_name).unwrap()))
    }

    fn create_local_image(
        &self,
        img_data: &ImgData,
        share_handles: UniquePtr<ShareHandles>,
    ) -> Result<ImageData, Box<dyn std::error::Error>> {
        let shmem = IpcShmem::new(
            &ImgData::convert_shmem_array_to_str(&img_data.shmem_name),
            &ImgData::convert_shmem_array_to_str(&img_data.name),
            false,
        )?;

        let vk_shared_image = {
            let rlock = shmem.acquire_rlock(raw_sync::Timeout::Val(Client::IPC_TIMEOUT))?;
            let rdata = IpcShmem::acquire_rdata(&rlock);

            let mut vk_shared_image: UniquePtr<VkSharedImage> = vk_shared_image_new();
            vk_shared_image.as_mut().unwrap().import_from_handle(
                self.vk_setup.get_vk_device(),
                self.vk_setup.get_vk_physical_device(),
                share_handles,
                &SharedImageData::from_shmem_img_data(rdata),
            );
            vk_shared_image
        };

        Ok(ImageData {
            ipc_info: shmem,
            vk_shared_image,
        })
    }

    fn find_image_internal(
        &mut self,
        image_name: &str,
        force_update: bool,
    ) -> Result<Option<&ImageData>, Box<dyn std::error::Error>> {
        if force_update {
            let res = self.find_image_cmd(image_name)?;
            return Ok(res);
        }

        let res = match self.shared_images.contains_key(image_name) {
            true => self.shared_images.get(image_name),
            false => self.find_image_cmd(image_name)?,
        };

        Ok(res)
    }

    fn find_image_cmd(
        &mut self,
        image_name: &str,
    ) -> Result<Option<&ImageData>, Box<dyn std::error::Error>> {
        let cmd_dat = ManuallyDrop::new(CommFindImage {
            image_name: ImgData::convert_shmem_str_to_array(image_name),
        });
        let cmd_msg = CommandMsg {
            tag: CommandTag::FindImage,
            data: CommandData { find_img: cmd_dat },
        };
        self.connection.send_command(cmd_msg)?;

        let res_msg = self.connection.recv_result()?;
        let res_data: Option<&ImgData> = match &res_msg {
            None => Ok(None),
            Some(msg) => match msg.tag {
                CommandTag::FindImage => {
                    let data = unsafe { &msg.data.find_img };
                    if data.image_found {
                        Ok(Some(&data.img_data))
                    } else {
                        Ok(None)
                    }
                }
                _ => Err(Box::new(Error::new(
                    ErrorKind::InvalidData,
                    "Received invalid data from server",
                ))),
            },
        }?;

        if res_data.is_none() {
            return Ok(None);
        }

        let res_data = res_data.unwrap();

        let mut share_handles = self.connection.recv_ancillary(1)?;

        self.connection.send_ack()?;

        let share_handles = vk_share_handles_from_fd(share_handles.pop().unwrap().into_raw_fd());

        let image_data = self.create_local_image(&res_data, share_handles)?;
        let local_image = self.shared_images.get_mut(image_name);
        if local_image.is_none() {
            self.shared_images
                .insert(image_name.to_string(), image_data);
        } else {
            *(local_image.unwrap()) = image_data;
        };

        Ok(Some(&self.shared_images.get(image_name).unwrap()))
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;
    use std::{fs, thread};

    use crate::platform::linux::ipc_unix_socket::IpcSocket;
    use crate::vulkan::vk_setup::ffi::vk_setup_new;

    use super::Client;

    const TIMEOUT: Duration = Duration::from_millis(2000);
    const SOCKET_PATH: &str = "test_socket.sock";
    const SHMEM_PREFIX: &str = "shared_images_";

    fn _create_server_socket() -> IpcSocket {
        IpcSocket::new(SOCKET_PATH, TIMEOUT).unwrap()
    }

    #[test]
    fn client_create() {
        let _ = fs::remove_file(SOCKET_PATH);

        let server_socket_fcn = || {
            let server_socket = _create_server_socket();
            server_socket.try_accept().unwrap()
        };

        let server_thread = thread::spawn(server_socket_fcn);

        let mut vk_setup = vk_setup_new();
        vk_setup.as_mut().unwrap().initialize_vulkan();

        let _client = Client::new(SOCKET_PATH, vk_setup, TIMEOUT).unwrap();

        let server_res = server_thread.join().unwrap();
        assert!(server_res.is_some());
    }
}
