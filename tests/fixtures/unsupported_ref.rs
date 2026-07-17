#![no_std]

#[unsafe(no_mangle)]
pub extern "C" fn sci_unsupported_ref_i32(value: i32) -> i32 {
    let value_ref = &value;
    *value_ref
}
