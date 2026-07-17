#![no_std]

#[repr(C)]
pub struct PairU64 {
    left: u64,
    right: u64,
}

unsafe extern "C" {
    fn sci_host_pair_arg(value: PairU64) -> u64;
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_abi_pair_arg(value: PairU64) -> u64 {
    value.left + value.right
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_abi_call_host_pair_arg(left: u64, right: u64) -> u64 {
    unsafe { sci_host_pair_arg(PairU64 { left, right }) }
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_abi_pair_return(left: u64, right: u64) -> PairU64 {
    PairU64 { left, right }
}
