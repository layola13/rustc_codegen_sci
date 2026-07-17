#![no_std]

#[unsafe(no_mangle)]
pub extern "Rust" fn sci_abi_pair_return(left: u64, right: u64) -> (u64, u64) {
    (left, right)
}
