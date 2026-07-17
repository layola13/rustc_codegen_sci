#![no_std]

unsafe extern "C" {
    fn sci_host_add_i32(a: i32, b: i32) -> i32;
    fn sci_host_note_i32(value: i32);
    fn sci_host_identity_ptr(value: *const i32) -> *const i32;
}

struct ScalarPair {
    left: i32,
    right: i32,
}

#[repr(C)]
pub struct FfiPair {
    left: i32,
    right: i32,
}

struct EmptyMarker;

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
pub extern "C" fn sci_tuple_sum_i32(a: i32, b: i32) -> i32 {
    let pair = (a, b);
    pair.0 + pair.1
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_tuple_copy_sum_i32(a: i32, b: i32) -> i32 {
    let pair = (a, b);
    let copy = pair;
    copy.0 + copy.1
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_struct_sum_i32(a: i32, b: i32) -> i32 {
    let pair = ScalarPair { left: a, right: b };
    pair.left + pair.right
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_struct_copy_sum_i32(a: i32, b: i32) -> i32 {
    let pair = ScalarPair { left: a, right: b };
    let copy = pair;
    copy.left + copy.right
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_empty_struct_local_i32(a: i32, b: i32) -> i32 {
    let _marker = EmptyMarker;
    a + b
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_stack_slot_i32(value: i32) -> i32 {
    let mut slot = value;
    let slot_ref = &mut slot;
    *slot_ref = 42;
    slot
}

#[unsafe(no_mangle)]
#[inline(never)]
pub extern "C" fn sci_identity_ptr(value: *const i32) -> *const i32 {
    value
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_call_identity_ptr(value: *const i32) -> *const i32 {
    sci_identity_ptr(value)
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_call_host_identity_ptr(value: *const i32) -> *const i32 {
    unsafe { sci_host_identity_ptr(value) }
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_ptr_eq(lhs: *const i32, rhs: *const i32) -> i32 {
    (lhs == rhs) as i32
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_ptr_is_null(value: *const i32) -> i32 {
    (value == core::ptr::null()) as i32
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_null_ptr() -> *const i32 {
    core::ptr::null()
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_load_i32(value: *const i32) -> i32 {
    unsafe { *value }
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_store_i32(slot: *mut i32, value: i32) {
    unsafe {
        *slot = value;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_replace_i32(slot: *mut i32, value: i32) -> i32 {
    let old = unsafe { *slot };
    unsafe {
        *slot = value;
    }
    old
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_load_pair_right(pair: *const FfiPair) -> i32 {
    unsafe { (*pair).right }
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_store_pair_left(pair: *mut FfiPair, value: i32) {
    unsafe {
        (*pair).left = value;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_replace_pair_right(pair: *mut FfiPair, value: i32) -> i32 {
    let old = unsafe { (*pair).right };
    unsafe {
        (*pair).right = value;
    }
    old
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_load_array_i32_at2(values: *const [i32; 4]) -> i32 {
    unsafe { (*values)[2] }
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_store_array_i32_at1(values: *mut [i32; 4], value: i32) {
    unsafe {
        (*values)[1] = value;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_replace_array_i32_at3(values: *mut [i32; 4], value: i32) -> i32 {
    let old = unsafe { (*values)[3] };
    unsafe {
        (*values)[3] = value;
    }
    old
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
pub extern "C" fn sci_call_fn_ptr_i32(
    f: extern "C" fn(i32, i32) -> i32,
    a: i32,
    b: i32,
) -> i32 {
    f(a, b)
}

#[unsafe(no_mangle)]
#[inline(never)]
pub extern "C" fn sci_unit_noop(_a: i32) {}

#[unsafe(no_mangle)]
pub extern "C" fn sci_call_unit_noop(a: i32) -> i32 {
    sci_unit_noop(a);
    42
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_call_host_note_i32(a: i32) -> i32 {
    unsafe { sci_host_note_i32(a) };
    42
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
pub extern "C" fn sci_mul_i64(a: i64, b: i64) -> i64 {
    a * b
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_mul_u64(a: u64, b: u64) -> u64 {
    a * b
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_add_usize(a: usize, b: usize) -> usize {
    a + b
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_mul_usize(a: usize, b: usize) -> usize {
    a * b
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_gt_isize(a: isize, b: isize) -> isize {
    (a > b) as isize
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

#[unsafe(no_mangle)]
pub extern "C" fn sci_match_i32(a: i32) -> i32 {
    match a {
        -7 => 7,
        -1 => 1,
        0 => 0,
        _ => -42,
    }
}
