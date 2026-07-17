#![no_std]

#[repr(C)]
pub struct OneU64 {
    value: u64,
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_abi_cast_return(value: u64) -> OneU64 {
    OneU64 { value }
}
