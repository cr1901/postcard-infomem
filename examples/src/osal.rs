//! OS abstraction layer for separating OS vs no OS versions of the example. 

use super::prelude::{*, osal::*};

cfg_if! {
    if #[cfg(any(target_os = "none", target_os="unknown"))] {
        pub trait OurCoreWrite: ehal::serial::Write {}
        impl<T> OurCoreWrite for T where T: ehal::serial::Write {}

        impl<Error> fmt::Write for dyn OurCoreWrite<Error = Error> where Error: ehal::serial::Error {
            fn write_str(&mut self, s: &str) -> Result<(), core::fmt::Error> {
                self.write(s.as_bytes()).map_err(|_| core::fmt::Error)
            }
        }
    } else {
        pub fn mk_writer() -> impl io::Write {
            io::stdout()
        }

        pub fn mk_reader(infomem: Slice) -> impl ReadSingle + IntoIterator<Item = u8> + Clone {
            infomem
        }
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
