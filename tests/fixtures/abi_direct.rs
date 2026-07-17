#![no_std]

unsafe extern "C" {
    fn sci_host_abi_i8_sub(a: i8, b: i8) -> i8;
    fn sci_host_abi_u8_xor(a: u8, b: u8) -> u8;
    fn sci_host_abi_i16_add(a: i16, b: i16) -> i16;
    fn sci_host_abi_u16_or(a: u16, b: u16) -> u16;
    fn sci_host_abi_i32_mul(a: i32, b: i32) -> i32;
    fn sci_host_abi_u32_and(a: u32, b: u32) -> u32;
    fn sci_host_abi_i64_sub(a: i64, b: i64) -> i64;
    fn sci_host_abi_u64_add(a: u64, b: u64) -> u64;
    fn sci_host_abi_isize_gt(a: isize, b: isize) -> isize;
    fn sci_host_abi_usize_mul(a: usize, b: usize) -> usize;
    fn sci_host_abi_ptr_identity(value: *const i32) -> *const i32;
    fn sci_host_abi_note_i32(value: i32);
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_abi_i8_add(a: i8, b: i8) -> i8 {
    a + b
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_abi_u8_xor(a: u8, b: u8) -> u8 {
    a ^ b
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_abi_i16_sub(a: i16, b: i16) -> i16 {
    a - b
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_abi_u16_or(a: u16, b: u16) -> u16 {
    a | b
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_abi_i32_mul(a: i32, b: i32) -> i32 {
    a * b
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_abi_u32_and(a: u32, b: u32) -> u32 {
    a & b
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_abi_i64_add(a: i64, b: i64) -> i64 {
    a + b
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_abi_u64_sub(a: u64, b: u64) -> u64 {
    a - b
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_abi_isize_gt(a: isize, b: isize) -> isize {
    (a > b) as isize
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_abi_usize_add(a: usize, b: usize) -> usize {
    a + b
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_abi_ptr_identity(value: *const i32) -> *const i32 {
    value
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_abi_void_noop(_value: i32) {}

#[unsafe(no_mangle)]
pub extern "C" fn sci_abi_call_host_i8_sub(a: i8, b: i8) -> i8 {
    unsafe { sci_host_abi_i8_sub(a, b) }
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_abi_call_host_u8_xor(a: u8, b: u8) -> u8 {
    unsafe { sci_host_abi_u8_xor(a, b) }
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_abi_call_host_i16_add(a: i16, b: i16) -> i16 {
    unsafe { sci_host_abi_i16_add(a, b) }
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_abi_call_host_u16_or(a: u16, b: u16) -> u16 {
    unsafe { sci_host_abi_u16_or(a, b) }
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_abi_call_host_i32_mul(a: i32, b: i32) -> i32 {
    unsafe { sci_host_abi_i32_mul(a, b) }
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_abi_call_host_u32_and(a: u32, b: u32) -> u32 {
    unsafe { sci_host_abi_u32_and(a, b) }
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_abi_call_host_i64_sub(a: i64, b: i64) -> i64 {
    unsafe { sci_host_abi_i64_sub(a, b) }
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_abi_call_host_u64_add(a: u64, b: u64) -> u64 {
    unsafe { sci_host_abi_u64_add(a, b) }
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_abi_call_host_isize_gt(a: isize, b: isize) -> isize {
    unsafe { sci_host_abi_isize_gt(a, b) }
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_abi_call_host_usize_mul(a: usize, b: usize) -> usize {
    unsafe { sci_host_abi_usize_mul(a, b) }
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_abi_call_host_ptr_identity(value: *const i32) -> *const i32 {
    unsafe { sci_host_abi_ptr_identity(value) }
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_abi_call_host_note_i32(value: i32) -> i32 {
    unsafe { sci_host_abi_note_i32(value) };
    99
}
