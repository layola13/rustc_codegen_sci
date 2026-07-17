#![no_std]

#[repr(C)]
pub struct OneI8 {
    value: i8,
}

#[repr(C)]
pub struct OneU8 {
    value: u8,
}

#[repr(C)]
pub struct OneI16 {
    value: i16,
}

#[repr(C)]
pub struct OneU16 {
    value: u16,
}

#[repr(C)]
pub struct OneI32 {
    value: i32,
}

#[repr(C)]
pub struct OneU32 {
    value: u32,
}

#[repr(C)]
pub struct OneI64 {
    value: i64,
}

#[repr(C)]
pub struct OneU64 {
    value: u64,
}

unsafe extern "C" {
    fn sci_host_cast_arg_i8(value: OneI8) -> i8;
    fn sci_host_cast_arg_u8(value: OneU8) -> u8;
    fn sci_host_cast_arg_i16(value: OneI16) -> i16;
    fn sci_host_cast_arg_u16(value: OneU16) -> u16;
    fn sci_host_cast_arg_i32(value: OneI32) -> i32;
    fn sci_host_cast_arg_u32(value: OneU32) -> u32;
    fn sci_host_cast_arg_i64(value: OneI64) -> i64;
    fn sci_host_cast_arg_u64(value: OneU64) -> u64;
    fn sci_host_cast_return_i8(value: i8) -> OneI8;
    fn sci_host_cast_return_u8(value: u8) -> OneU8;
    fn sci_host_cast_return_i16(value: i16) -> OneI16;
    fn sci_host_cast_return_u16(value: u16) -> OneU16;
    fn sci_host_cast_return_i32(value: i32) -> OneI32;
    fn sci_host_cast_return_u32(value: u32) -> OneU32;
    fn sci_host_cast_return_i64(value: i64) -> OneI64;
    fn sci_host_cast_return_u64(value: u64) -> OneU64;
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_abi_cast_return_i8(value: i8) -> OneI8 {
    OneI8 { value }
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_abi_cast_return_u8(value: u8) -> OneU8 {
    OneU8 { value }
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_abi_cast_return_i16(value: i16) -> OneI16 {
    OneI16 { value }
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_abi_cast_return_u16(value: u16) -> OneU16 {
    OneU16 { value }
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_abi_cast_return_i32(value: i32) -> OneI32 {
    OneI32 { value }
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_abi_cast_return_u32(value: u32) -> OneU32 {
    OneU32 { value }
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_abi_cast_return_i64(value: i64) -> OneI64 {
    OneI64 { value }
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_abi_cast_return(value: u64) -> OneU64 {
    OneU64 { value }
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_abi_cast_arg_i8(value: OneI8) -> i8 {
    value.value
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_abi_cast_arg_u8(value: OneU8) -> u8 {
    value.value
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_abi_cast_arg_i16(value: OneI16) -> i16 {
    value.value
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_abi_cast_arg_u16(value: OneU16) -> u16 {
    value.value
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_abi_cast_arg_i32(value: OneI32) -> i32 {
    value.value
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_abi_cast_arg_u32(value: OneU32) -> u32 {
    value.value
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_abi_cast_arg_i64(value: OneI64) -> i64 {
    value.value
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_abi_cast_arg_u64(value: OneU64) -> u64 {
    value.value
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_abi_call_host_cast_arg_i8(value: i8) -> i8 {
    unsafe { sci_host_cast_arg_i8(OneI8 { value }) }
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_abi_call_host_cast_arg_u8(value: u8) -> u8 {
    unsafe { sci_host_cast_arg_u8(OneU8 { value }) }
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_abi_call_host_cast_arg_i16(value: i16) -> i16 {
    unsafe { sci_host_cast_arg_i16(OneI16 { value }) }
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_abi_call_host_cast_arg_u16(value: u16) -> u16 {
    unsafe { sci_host_cast_arg_u16(OneU16 { value }) }
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_abi_call_host_cast_arg_i32(value: i32) -> i32 {
    unsafe { sci_host_cast_arg_i32(OneI32 { value }) }
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_abi_call_host_cast_arg_u32(value: u32) -> u32 {
    unsafe { sci_host_cast_arg_u32(OneU32 { value }) }
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_abi_call_host_cast_arg_i64(value: i64) -> i64 {
    unsafe { sci_host_cast_arg_i64(OneI64 { value }) }
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_abi_call_host_cast_arg_u64(value: u64) -> u64 {
    unsafe { sci_host_cast_arg_u64(OneU64 { value }) }
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_abi_call_host_cast_return_i8(value: i8) -> i8 {
    unsafe { sci_host_cast_return_i8(value).value }
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_abi_call_host_cast_return_u8(value: u8) -> u8 {
    unsafe { sci_host_cast_return_u8(value).value }
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_abi_call_host_cast_return_i16(value: i16) -> i16 {
    unsafe { sci_host_cast_return_i16(value).value }
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_abi_call_host_cast_return_u16(value: u16) -> u16 {
    unsafe { sci_host_cast_return_u16(value).value }
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_abi_call_host_cast_return_i32(value: i32) -> i32 {
    unsafe { sci_host_cast_return_i32(value).value }
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_abi_call_host_cast_return_u32(value: u32) -> u32 {
    unsafe { sci_host_cast_return_u32(value).value }
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_abi_call_host_cast_return_i64(value: i64) -> i64 {
    unsafe { sci_host_cast_return_i64(value).value }
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_abi_call_host_cast_return_u64(value: u64) -> u64 {
    unsafe { sci_host_cast_return_u64(value).value }
}
