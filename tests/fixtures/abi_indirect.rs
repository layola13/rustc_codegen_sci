#![no_std]

#[repr(C)]
pub struct ThreeU64 {
    first: u64,
    second: u64,
    third: u64,
}

#[unsafe(no_mangle)]
pub extern "C" fn sci_abi_indirect_return(first: u64, second: u64, third: u64) -> ThreeU64 {
    ThreeU64 {
        first,
        second,
        third,
    }
}
