use crate::platform::ShmemDataInternal;

#[no_mangle]
extern "C" fn shmem_data_internal_default() -> ShmemDataInternal {
    ShmemDataInternal::default()
}
