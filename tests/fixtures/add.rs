#![no_std]

unsafe extern "C" {
    fn sci_host_add_i32(a: i32, b: i32) -> i32;
}

#[unsafe(no_mangle)]
#[inline(never)]
pub extern "C" fn sci_add_i32(a: i32, b: i32) -> i32 {
    a + b
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_gt_i32(a: i32, b: i32) -> i32 {
    (a > b) as i32
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_max_i32(a: i32, b: i32) -> i32 {
    if a > b { a } else { b }
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_call_add_i32(a: i32, b: i32) -> i32 {
    sci_add_i32(a, b)
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_call_host_add_i32(a: i32, b: i32) -> i32 {
    unsafe { sci_host_add_i32(a, b) }
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_sub_i32(a: i32, b: i32) -> i32 {
    a - b
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_mul_i32(a: i32, b: i32) -> i32 {
    a * b
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_div_i32(a: i32, b: i32) -> i32 {
    a / b
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_rem_i32(a: i32, b: i32) -> i32 {
    a % b
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_shl_i32(a: i32, b: i32) -> i32 {
    a << b
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_shr_i32(a: i32, b: i32) -> i32 {
    a >> b
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_neg_i32(a: i32) -> i32 {
    -a
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_not_i32(a: i32) -> i32 {
    !a
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_match_u32(a: u32) -> i32 {
    match a {
        0 => 7,
        1 => 11,
        42 => 42,
        _ => -1,
    }
}
