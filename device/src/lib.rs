/*! Helper crate for [`InfoMem`](../postcard_infomem/struct.InfoMem.html) `struct`s
intended to primarily be used in `no_std` environments (although this does not
preclude using the crate for hosted applications).
*/
#![no_std]

#[macro_export]
/** Create a `static` variable to hold a serialized [`InfoMem`](../postcard_infomem/struct.InfoMem.html) structure.

This macro can be invoked in one of three ways:

* ```ignore
  include_postcard_infomem!("/path/to/binary/infomem/file", ".linker-section", VAR_NAME);
  ```
* ```ignore
  include_postcard_infomem!("/path/to/binary/infomem/file", ".linker-section");
  ```
* ```ignore
  include_postcard_infomem!("/path/to/binary/infomem/file");
  ```

If `".linker-section"` is omitted, it defaults to `".info"`, and if `VAR_NAME` is
omitted, the `static` variable's name defaults to `INFOMEM`. The `static`
variable generated will have a type of "the dereferenced return value of
[`include_bytes`]".

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
        #[cfg(not(target_arch = "avr"))]
        include_postcard_infomem!($pim, ".info", INFOMEM);

        #[cfg(target_arch = "avr")]
        include_postcard_infomem!($pim, avr_infomem);
    };

    ($pim:expr, $sec_or_mod:literal) => {
        #[cfg(not(target_arch = "avr"))]
        include_postcard_infomem!($pim, $sec_or_mod, INFOMEM);

        #[cfg(target_arch = "avr")]
        compile_error!(
            concat!(
                "on AVR, the two-argument version of include_postcard_infomem accepts",
                " an ident as the second argument (received literal)"
            )
        );
    };

    ($pim:expr, $mod:ident) => {
        #[cfg(not(target_arch = "avr"))]
        compile_error!(
            concat!(
                "on non-AVR targets, the two-argument version of include_postcard_infomem accepts",
                " a literal as the second argument (received ident)"
            )
        );

        /* AVR stores EEPROM in a separate address space. Access the variable
        INFOMEM from code will try to access at the same offset in a
        different address space. This is a spatial memory-safety violation.
        Avoid the problem by not allowing users to access the variable
        directly.

        We turn off no_mangle because the $mod argument serves that purpose
        at compile-time (this was multiple payloads can be included if desired).

        TODO: Provide some sort of API to iterate over INFOMEM. */
        pub mod $mod {
            #[link_section = ".eeprom"]
            #[used]
            static INFOMEM: [u8; include_bytes!($pim).len()] = *include_bytes!($pim);

            /* Abuse the fact that INFOMEM is pointing into the wrong address
               space to generate an usize to supply to the EEPROM registers. */
            pub fn start() -> usize {
                &INFOMEM as *const u8 as usize
            }

            pub fn len() -> usize {
                INFOMEM.len()
            }

            pub fn end() -> usize {
                start() + len()
            }
        }
    };

    ($pim:expr, $sec:literal, $var_name:ident) => {
        #[cfg(not(target_arch = "avr"))]
        #[link_section = $sec]
        #[used]
        #[no_mangle]
        static $var_name: [u8; include_bytes!($pim).len()] = *include_bytes!($pim);

        #[cfg(target_arch = "avr")]
        compile_error!(
            "only the single or two-argument version of include_postcard_infomem is supported on AVR"
        );
    };
}

#[cfg(test)]
mod tests {}
