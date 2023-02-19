#![cfg_attr(any(target_os = "none", target_os = "unknown"), no_std)]
#![cfg_attr(any(target_os = "none", target_os = "unknown"), no_main)]

mod prelude;
// use prelude::Error;
mod hal;
use hal::*;

use core::ops::Range;

use postcard_infomem::{InfoMem, de::Magic, from_seq_magic};

use postcard_infomem_device::include_postcard_infomem;
include_postcard_infomem!(concat!(env!("OUT_DIR"), "/info.bin"));

/* On hosted archs, this is the actual main() function. But on embedded apps
without an OS (`no_main`), this main() is called by the true main() function.
See prelude module. */
fn main() {
    output_init();

    let mut buf = [0; 128];

    write("\r\ncontents\r\n");
    
    for data in mk_reader(infomem::get()) {
        write_hex(data.unwrap());
    }

    write("\r\ndeserialize\r\n");

    let im_reader = mk_reader(infomem::get());

    match from_seq_magic::<_, _, &[u8]>(im_reader, &mut buf) {
        Ok(im) => {
            write("\r\nokay\r\n");
        }
        Err(e) => {
            write("\r\nerror: ");
            write_hex(e as u8);
            write("\r\n");
        }
    }

    // for data in im_reader {
    //     write(data.unwrap());
    // }
    // let im = from_bytes_magic(s).unwrap()
}

