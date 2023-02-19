#![cfg_attr(any(target_os = "none", target_os = "unknown"), no_std)]
#![cfg_attr(any(target_os = "none", target_os = "unknown"), no_main)]

mod prelude;
use prelude::*;

mod hal;
use hal::*;

use postcard_infomem::from_seq_magic;

use postcard_infomem_device::include_postcard_infomem;
include_postcard_infomem!(concat!(env!("OUT_DIR"), "/info.bin"));

/* On hosted archs, this is the actual main() function. But on embedded apps
without an OS (`no_main`), this main() is called by the true main() function.
See prelude module. */
fn main() {
    let mut w = mk_writer();
    #[cfg(any(target_os = "none", target_os = "unknown"))]
    let w: &mut dyn OurCoreWrite<Error = _> = &mut w;

    let mut buf = [0; 128];

    write!(w, "\r\nDumping infomem contents...\r\n").unwrap();
    
    for data in mk_reader(infomem::get()) {
        #[cfg(any(target_os = "none", target_os = "unknown"))]
        write!(w, "{}", Ascii::from(data)).unwrap();
        #[cfg(all(not(target_os = "none"), not(target_os="unknown")))]
        write!(w, "{}", Ascii::from(*data)).unwrap();
    }

    write!(w, "\r\n\r\nDeserializing infomem... ").unwrap();

    let im_reader = mk_reader(infomem::get());

    match from_seq_magic::<_, _, &[u8]>(im_reader, &mut buf) {
        Ok(_im) => {
            write!(w, "Okay!\r\n").unwrap();
        }
        Err(e) => {
            write!(w, "Error: {}\r\n", e as u8).unwrap();
        }
    }
}

