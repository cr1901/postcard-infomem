#![no_std]
#![no_main]

extern crate panic_msp430;
use msp430_rt::entry;

#[allow(unused)]
use msp430g2553::interrupt;

use postcard_infomem_device::include_postcard_infomem;

include_postcard_infomem!(concat!(env!("OUT_DIR"), "/info.bin"));

#[entry]
fn main() -> ! {
    loop {}
}

#[no_mangle]
extern "C" fn abort() -> ! {
    panic!();
}
