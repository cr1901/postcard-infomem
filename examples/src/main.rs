#![cfg_attr(any(target_os = "none", target_os = "unknown"), no_std)]
#![cfg_attr(any(target_os = "none", target_os = "unknown"), no_main)]

mod prelude;
use prelude::*;
#[allow(unused_imports)]
use prelude::osal::*;

mod hal;
#[allow(unused_imports)]
use hal::*;

mod osal;
use osal::*;

include_postcard_infomem!(concat!(env!("OUT_DIR"), "/info.bin"));

pub struct Ascii(u8);

impl From<u8> for Ascii {
    fn from(value: u8) -> Self {
        Self(value)
    }
}

impl fmt::Display for Ascii {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0 < 0x20 || self.0 > 127 {
            write!(f, "\\x{:02X}", self.0)?;
        } else {
            write!(f, "{}", self.0 as char)?;
        }

        Ok(())
    }
}

/* On hosted archs, this is the actual main() function. But on embedded apps
without an OS (`no_main`), this main() is called by the true main() function.
See prelude module. */
fn main() {
    let mut w = mk_writer();
    #[cfg(any(target_os = "none", target_os = "unknown"))]
    let w: &mut dyn OurCoreWrite<Error = _> = &mut w;
    let r = mk_reader(infomem::get());
    let iter = mk_iterator(r.clone());

    let mut buf = [0u8; 128];

    write!(w, "\r\nDumping infomem contents...\r\n").unwrap();

    for data in iter {
        write!(w, "{}", Ascii::from(data)).unwrap();
    }

    write!(w, "\r\n\r\nDeserializing infomem... ").unwrap();

    match deserialize_infomem(r, &mut buf) {
        Ok(_im) => {
            write!(w, "Okay!\r\n").unwrap();
        }
        Err(e) => {
            write!(w, "Error: {}\r\n", e as u8).unwrap();
        }
    }
}
