#![feature(unix_socket_ancillary_data)]
//#![allow(dead_code, unused_imports)]

pub mod platform;
mod shared_image;
pub mod vulkan;

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
