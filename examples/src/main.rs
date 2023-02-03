#![no_std]
#![no_main]

use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(feature = "msp430g2553")] {
        extern crate panic_msp430;
        use msp430_rt::entry;

        #[allow(unused)]
        use msp430g2553::interrupt;
    } else if #[cfg(feature = "rp2040-hal")] {
        extern crate panic_halt;
    }
}

use postcard_infomem_device::include_postcard_infomem;
include_postcard_infomem!(concat!(env!("OUT_DIR"), "/info.bin"));

#[cfg_attr(feature = "msp430g2553", entry)]
fn main() -> ! {
    loop {}
}

#[no_mangle]
extern "C" fn abort() -> ! {
    panic!();
}
