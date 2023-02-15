/*! Helper crate for [`InfoMem`](../postcard_infomem/struct.InfoMem.html) `struct`s
intended to primarily be used in `no_std` environments (although this does not
preclude using the crate for hosted applications).
*/
#![no_std]

#[macro_export]
/** Create a `static` variable to hold a serialized [`InfoMem`](../postcard_infomem/struct.InfoMem.html) structure.

This macro can be invoked in one of two ways:

* ```ignore
  # use postcard_infomem_device::include_postcard_infomem;
  include_postcard_infomem!("/path/to/binary/infomem/file", generated_module_name);
  ```
* ```ignore
  # use postcard_infomem_device::include_postcard_infomem;
  include_postcard_infomem!("/path/to/binary/infomem/file");
  ```

If `generated_module_name` is omitted, it defaults to `infomem`.

On [Harvard architectures](https://en.wikipedia.org/wiki/Harvard_architecture)
like AVR, information memory may be stored in a separate address space. Accessing
information memory as if it was in the same address space for program data would
lead to a spatial memory safety violation. Therefore, the generated `static`
cannot be accessed directly. To access the `static` serialized byte array,
use the generic `generated_module_name::get()`:

`get()` can return a `&[u8]` on most architectures supported by Rust:

```ignore
# use postcard_infomem_device::include_postcard_infomem;
#
include_postcard_infomem!("/path/to/binary/infomem/file");
let im: &[u8] = infomem::get();
```

On all supported architectures where [`usize`] is the size of a [`pointer`],
`get()` can also return a `Range<usize>` suitable for iterating. The start
and end pointers of the serialized `static` are interpreted as [`usize`]s. These
will be replaced with an int type guaranteed to be the size of a [`pointer`],
[eventually](https://github.com/rust-lang/rust/issues/65473):

```ignore
# use postcard_infomem_device::include_postcard_infomem;
#
# fn read_infomem_data(addr)
# {
# }
include_postcard_infomem!("/path/to/binary/infomem/file");

let addrs = infomem::get::<Range<usize>>();
for a in addrs {
    println!("{}", read_infomem_data(a))
}
```

## Linker Considerations.
The generated `static` variable is annotated with the [`link_section` attribute](https://doc.rust-lang.org/reference/abi.html#the-link_section-attribute).
Currently, on all targets except the AVR, the link section is named `.postcard_infomem`.
On AVR, the link section is named `.eeprom`; _the `avr-gcc` toolchain has special
logic to place sections named `.eeprom` into EEPROM memory._

This macro also annotates the `static` variable with the [`used` attribute](https://doc.rust-lang.org/reference/abi.html#the-used-attribute)
so that `rustc` knows not to optimize the variable away if your application
never reads from it. However, linkers _also_ have a tendency to [garbage-collect](https://sourceware.org/binutils/docs/ld/Input-Section-Keep.html)
unused symbols unless told not to. The [`postcard-infomem-host`](../postcard-infomem-host/index.html) crate provides
functions for build scripts to automate generating these linker fragments for you.

Complete/working examples of using this macro based on the above can be found in
the `examples` directory/[crate](https://github.com/cr1901/postcard-infomem/tree/main/examples)
of this workspace.
*/
macro_rules! include_postcard_infomem {
    ($pim:expr) => {
        include_postcard_infomem!($pim, infomem);
    };

    ($pim:expr, $mod:ident) => {
        /* AVR stores EEPROM in a separate address space. Access the variable
        INFOMEM from code will try to access at the same offset in a
        different address space. This is a spatial memory-safety violation.
        Avoid the problem by not allowing users to access the variable
        directly.

        We turn on no_mangle because multiple INFOMEMs are not
        supported at this time. */
        pub mod $mod {
            use core::ops::Range;
            use core::slice::from_raw_parts;

            #[cfg(not(doctest))]
            #[cfg_attr(target_arch = "avr", link_section = ".eeprom")]
            #[cfg_attr(not(target_arch = "avr"), link_section = ".postcard_infomem")]
            #[no_mangle]
            #[used]
            static INFOMEM: [u8; include_bytes!($pim).len()] = *include_bytes!($pim);

            // Doesn't seem to work...
            #[cfg(doctest)]
            static INFOMEM: [u8; 7] = b"doctest";

            #[doc="Pointer to an infomem struct that is address-space aware.\
            \n\
            The [`Ptr`] struct is a wrapper over a pointer that can be used as \
            a portable type to access the `INFOMEM` `static` on targets with one or \
            multiple address spaces. \
            \n\
            It is meant to be immediately converted to another, more ergonomic type for \
            your environment."]
            pub struct Ptr(*const u8, usize);

            #[doc="Access information memory safely, depending on target. \
            \n\
            This can be used as a portable entry point to access the `INFOMEM`
            `static` on targets with one or multiple address spaces.\n\
            On most targets, [`From<Ptr>`] will be defined for &[u8], which points \
            to the `INFOMEM` `static`. On all targets, [`From<Ptr>`] is defined for \
            [`Range<usize>`], which can iterate over `usize`s representing each address \
            used by the `INFOMEM` `struct`."]
            pub fn get<T>() -> T where T: From<Ptr> {
                Ptr(INFOMEM.as_ptr(), INFOMEM.len()).into()
            }

            #[cfg(not(target_arch = "avr"))]
            impl From<Ptr> for &[u8] {
                fn from(value: Ptr) -> Self {
                    // SAFETY: Ptrs can only be created within this module.
                    // It is derived from a static array with known length.
                    unsafe { from_raw_parts(value.0, value.1) }
                }
            }

            impl From<Ptr> for Range<usize> {
                fn from(value: Ptr) -> Self {
                    Range {
                        start: value.0 as usize,
                        end: value.0 as usize + value.1
                    }
                }
            }
        }
    };
}

#[cfg(test)]
mod tests {}
