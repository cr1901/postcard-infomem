#![no_std]
#![no_main]

use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(feature = "msp430g2553")] {
        extern crate panic_msp430;
        use msp430_rt::entry;
        use msp430g2553::interrupt as _;
    } else if #[cfg(feature = "rp2040-hal")] {
        extern crate panic_halt;
        use rp2040_hal::entry;

        /// The linker will place this boot block at the start of our program image. We
        /// need this to help the ROM bootloader get our code up and running.
        #[link_section = ".boot2"]
        #[no_mangle]
        #[used]
        pub static BOOT2_FIRMWARE: [u8; 256] = rp2040_boot2::BOOT_LOADER_GD25Q64CS;
    }
}

use postcard_infomem_device::include_postcard_infomem;
include_postcard_infomem!(concat!(env!("OUT_DIR"), "/info.bin"));

#[cfg_attr(any(feature = "msp430g2553", feature = "rp2040-hal"), entry)]
fn main() -> ! {
    loop {}
}

#[no_mangle]
extern "C" fn abort() -> ! {
    panic!();
}
