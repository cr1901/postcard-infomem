pub use cfg_if::cfg_if;

pub use core::fmt;
pub use core::ops;

pub use postcard_infomem::{ReadSingle, ReadSingleError};
pub use postcard_infomem_device::*;

// No OS (embedded apps) vs OS
cfg_if! {
    if #[cfg(any(target_os = "none", target_os = "unknown"))] {
        pub use core::fmt::write;
        pub use core::fmt::Write;
        pub use core::convert::Infallible;
    } else {
        pub use std::io;
        pub use std::io::Write;
    }
}

// For No OS, target-specific imports.
cfg_if! {
    if #[cfg(feature = "msp430g2553")] {
        extern crate panic_msp430;
        pub use msp430_rt::entry;

        #[allow(unused)]
        use msp430g2553::interrupt;
    } else if #[cfg(feature = "rp2040-hal")] {
        extern crate panic_halt;
        pub use rp2040_hal::entry;

        /// The linker will place this boot block at the start of our program image. We
        /// need this to help the ROM bootloader get our code up and running.
        #[link_section = ".boot2"]
        #[no_mangle]
        #[used]
        pub static BOOT2_FIRMWARE: [u8; 256] = rp2040_boot2::BOOT_LOADER_GD25Q64CS;
    } else if #[cfg(feature = "ruduino")] {
        pub use ruduino;
        pub use ruduino::cores::current::{port, EEAR, EECR, EEDR};
        pub use ruduino::legacy::serial;
        pub use ruduino::Pin;
        pub use ruduino::Register;

        pub const CPU_FREQUENCY_HZ: u64 = 16_000_000;
        pub const BAUD: u64 = 9600;
        pub const UBRR: u16 = (CPU_FREQUENCY_HZ / 16 / BAUD - 1) as u16;
    }
}

/* For embedded apps, these are various versions of the true entry point for
the linker.
For hosted apps, super::main() is the actual entry point. */

// Embedded Rust ecosystem
#[cfg(any(feature = "msp430g2553", feature = "rp2040-hal"))]
#[cfg_attr(any(feature = "msp430g2553", feature = "rp2040-hal"), entry)]
fn main() -> ! {
    super::main();

    loop {}
}

// AVR ecosystem
#[cfg(feature = "ruduino")]
#[no_mangle]
pub extern "C" fn main() {
    super::main();

    loop {}
}

// May be required in general, putting here just in case...
#[no_mangle]
extern "C" fn abort() -> ! {
    panic!();
}
