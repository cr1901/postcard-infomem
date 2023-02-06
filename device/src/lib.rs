/*! Helper crate for [`InfoMem`](../postcard_infomem/struct.InfoMem.html) `struct`s
intended to primarily be used in `no_std` environments (although this does not
preclude using the crate for hosted applications).
*/
#![no_std]

#[macro_export]
/** Create a `static` variable to hold a serialized [`InfoMem`](../postcard_infomem/struct.InfoMem.html) structure.

This macro can be invoked in one of three ways:

* ```
  include_postcard_infomem!("/path/to/binary/infomem/file", ".linker-section", VAR_NAME)
  ```
* ```
  include_postcard_infomem!("/path/to/binary/infomem/file", ".linker-section")
  ```
* ```
  include_postcard_infomem!("/path/to/binary/infomem/file")
  ```

If `".linker-section"` is omitted, it defaults to `".info"`, and if `VAR_NAME` is
omitted, the `static` variable's name defaults to `INFOMEM`.

This macro also generates two `const` variables:

* `INFOMEM_REF`: A `&[u8]` that is a reference to the output of the [`include_bytes`] macro.
* `INFOMEM_LEN`: A `usize` containing the length of `INFOMEM_REF`.

The `static` variable generated will have a type of `[u8; INFOMEM_LEN]`.

## Linker Considerations.
The macro annotates the `static` variable with the [`used` attribute](https://doc.rust-lang.org/reference/abi.html#the-used-attribute)
so that `rustc` knows not to optimize the variable away if your application
never reads from it. However, linkers _also_ have a tendency to [garbage-collect](https://sourceware.org/binutils/docs/ld/Input-Section-Keep.html)
unused symbols unless told not to.

For [GNU `ld`](https://sourceware.org/binutils/docs/ld/)-based linkers, working
around garbage collection involves overriding the default linker script by passing
the `-C link-arg=-T/path/to/linker/script/override` [codegen flag](https://doc.rust-lang.org/rustc/codegen-options/index.html#link-arg)
to `rustc`, and adding a `KEEP(.linker-section)` annotation inside the
aforementioned linker script override file. An example of an override for
[msp430g2553](https://docs.rs/msp430g2553/latest/msp430g2553/) might look like:

```text
MEMORY
{
  RAM : ORIGIN = 0x0200, LENGTH = 0x0200
  INFOMEM : ORIGIN = 0x1000, LENGTH = 0x100
  ROM : ORIGIN = 0xC000, LENGTH = 0x3FDE
  VECTORS : ORIGIN = 0xFFE0, LENGTH = 0x20
}

SECTIONS {
    .info : {
      _sinfo = .;
      KEEP(*(.info))
      _einfo = .;
    } > INFOMEM
}

/* This is a precaution. If you have a way to save the calibration data before
it's been erased, all 256-2 bytes of information memory can be used. */
ASSERT((_einfo - _sinfo) <= 192, "
ERROR(memory.x): Information memory is greater than 192 bytes long. Erasing flash
to write the information memory would also erase (and possibly overwrite) MSP430Gx2xx
calibration data.");
```

If you want to _extend_ a default linker script script, but not completely
override it, the `INSERT` [annotation](https://sourceware.org/binutils/docs/ld/Miscellaneous-Commands.html)
can be used _while still passing the `link-arg` codegen flag above._ This is
useful for appending an [`InfoMem`](../postcard_infomem/struct.InfoMem.html)
`struct` to the end of your binary's [`.text` section](https://en.wikipedia.org/wiki/Code_segment).
The below example was tested with [`mingw-w64`](https://www.mingw-w64.org/) and
a `rustc` targeting the [GNU ABI](https://rust-lang.github.io/rustup/installation/windows.html)
on Windows:

```text
SECTIONS {
    /* Required, otherwise won't be treated as valid Win32 app.. */
    . = ALIGN(__section_alignment__);
    .info : {
      _sinfo = .;
      KEEP(*(.info))
      _einfo = .;
    }
} INSERT AFTER .text
```

Complete/working examples of using this macro based on the above can be found
in the `examples` directory/[crate](https://github.com/cr1901/postcard-infomem/tree/main/examples)
of this workspace.
*/
macro_rules! include_postcard_infomem {
    ($pim:expr) => {
        include_postcard_infomem!($pim, ".info", INFOMEM);
    };

    ($pim:expr, $sec:literal) => {
        include_postcard_infomem!($pim, $sec, INFOMEM);
    };

    ($pim:expr, $sec:literal, $var_name:ident) => {
        const INFOMEM_REF: &[u8] = include_bytes!($pim);
        const INFOMEM_LEN: usize = INFOMEM_REF.len();

        #[link_section = $sec]
        #[used]
        #[no_mangle]
        static $var_name: [u8; INFOMEM_LEN] = {
            let mut arr = [0; INFOMEM_LEN];
            let mut idx = 0;

            while idx < INFOMEM_LEN {
                arr[idx] = INFOMEM_REF[idx];
                idx += 1;
            }

            arr
        };
    };
}

#[cfg(test)]
mod tests {}
