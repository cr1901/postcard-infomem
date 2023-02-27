/*! Helper crate for [`InfoMem`](../postcard_infomem/struct.InfoMem.html) `struct`s
intended to primarily be used in `no_std` environments (although this does not
preclude using the crate for hosted applications).
*/
#![no_std]

#[allow(unused_imports)]
use core::slice::from_raw_parts;
use core::ops;

use postcard_infomem::{SequentialRead, SequentialReadError};

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
            #[cfg(not(doctest))]
            #[cfg_attr(target_arch = "avr", link_section = ".eeprom")]
            #[cfg_attr(not(target_arch = "avr"), link_section = ".postcard_infomem")]
            #[no_mangle]
            #[used]
            static INFOMEM: [u8; include_bytes!($pim).len()] = *include_bytes!($pim);

            // Doesn't seem to work...
            #[cfg(doctest)]
            static INFOMEM: [u8; 7] = b"doctest";

            #[doc = "Access information memory safely, depending on target.\
            \n\
            This can be used as a portable entry point to access the `INFOMEM`
            `static` on targets with one or multiple address spaces.\n\
            On most targets, [`From<Ptr>`] will be defined for [`Slice`], which \
            is a wrapper over a &'static [u8] which contains the `INFOMEM` \
            `static`. On all targets, [`From<Ptr>`] is defined for [`Range<usize>`], \
            which can iterate over `usize`s representing each address used by \
            the `INFOMEM` `struct`."]
            pub fn get() -> $crate::InfoMemPtr 
            {
                // SAFETY: `InfoMemPtr` is derived from a static array with known length.
                unsafe { $crate::InfoMemPtr::new(INFOMEM.as_ptr() as usize, INFOMEM.as_ptr() as usize + INFOMEM.len()) }
            }
        }
    };
}

/** Pointer to an infomem struct that is address-space aware.

The [`Ptr`] `struct` is a wrapper over a pointer that can be used as
a portable type to access the `INFOMEM` `static` on targets with one or
multiple address spaces.

In most scenarios, `InfoMemPtr` can be immediately converted to a &`static [u8]
containing the `INFOMEM` contents. However, other scenarios exist where this is
not possible, such as Information Memory being stored in a separate address
space or a serial (e.g. I2C) EEPROM. In those cases, the [`sequential_read`]
function can be used. */
pub struct InfoMemPtr(ops::Range<usize>);

impl InfoMemPtr {
    /** Create an abstract pointer to an `INFOMEM` `static`.

    ## Safety

    Due to publicly-available [`From`] impls, the pointer needs to
    point to a valid memory block that's _not_ currently mutably borrowed.
    Additionally, `start` must be less than `end`.
    Otherwise, Undefined Behavior may occur when [`InfoMemPtr`] is converted.

    [`include_postcard_infomem`] takes care of safely creating an [`InfoMemPtr`] for you.
    */
    pub unsafe fn new(start: usize, end: usize) -> Self {
        Self(ops::Range { start, end })
    }

    /** Create an adapter from an [`InfoMemPtr`] to access sequentially access
    an `INFOMEM` not in the current address space. Return type implements
    [`SequentialRead`], [`Iterator`], and [`Clone`].

    This function is a convenience to avoid the need to create unique types
    for each platform that implement the [`SequentialRead`] trait. The return
    type also implements [`Iterator`] for parity with `&[u8]`; errors are
    mapped to [`None`] when used as an iterator.
    
    [`InfoMemPtr`] by itself contains [`usize`]s that represent generic addresses.
    The user creates a closure that takes addresses supplied by [`InfoMemPtr`]
    to access `INFOMEM` in a platform specific manner. For instance, the
    following two loops which print the contents of `INFOMEM` as [`UpperHex`]
    are equivalent:

    ```ignore
    for data in infomem::get().sequential_read(|addr| {
        // SAFTEY: we have to opt into unsafety to create an `InfoMemPtr`,
        // a range provided by include_postcard_infomem macro will be
        // safe to dereference over all `usize`s passed to this closure.
        Ok(unsafe { *(addr as *const u8) })
    }) {
        write!(w, "{:X}", data).unwrap();
    }
    ```

    ```ignore
    for data in <&'static [u8]>::from(infomem::get()) {
        write!(w, "{:X}", data).unwrap();
    }
    ``` 
    
    In practice, a closure passed to `sequential_read` will access I/O to
    stream information memory from a separate address space or off-chip
    peripheral.
    */
    pub fn sequential_read<F>(self, f: F) -> impl SequentialRead + Iterator<Item = u8> + Clone where F: FnMut(usize) -> Result<u8, SequentialReadError> + Clone {
        InfoMemSequentialRead(self.0, f)
    }
}

#[cfg(not(target_arch = "avr"))]
impl<'a> From<InfoMemPtr> for &'a [u8] {
    fn from(value: InfoMemPtr) -> Self {
        // SAFETY: You have already opted into `unsafe` to create
        // an [`InfoMemPtr`], and are upholding `InfoMemPtr`s invariants before
        // doing the conversion.
        unsafe { from_raw_parts(value.0.start as *const u8, value.0.end - value.0.start) }
    }
}

/* `struct` which maps [`usize`] addresses to `INFOMEM` contents outside of
the current data address space.

This `struct` is a convenience to avoid the need to create unique types
for each platform that implement the [`SequentialRead`] trait. It also implements
[`Iterator`] for parity with `&[u8]`; errors are mapped to [`None`] when
used as an iterator.
*/
#[derive(Clone)]
struct InfoMemSequentialRead<F>(ops::Range<usize>, F);

impl<F> SequentialRead for InfoMemSequentialRead<F> where F: FnMut(usize) -> Result<u8, SequentialReadError> {
    fn sequential_read(&mut self) -> Result<u8, SequentialReadError> {
        let addr = self.0.next().ok_or(SequentialReadError)?;
        (self.1)(addr)
    }
}

impl<F> Iterator for InfoMemSequentialRead<F> where F: FnMut(usize) -> Result<u8, SequentialReadError> {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        let addr = self.0.next()?;
        (self.1)(addr).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use postcard::{to_allocvec, from_bytes};
    use postcard_infomem::{InfoMem, to_allocvec_magic, from_seq_magic};

    extern crate std;
    use std::vec::Vec;

    #[test]
    fn test_range_sequential_read_slice_equiv() {
        let im_ser = to_allocvec_magic(&InfoMem::<&[u8]>::default()).unwrap().leak();
        let (start, end) = (im_ser.as_ptr() as usize, im_ser.as_ptr() as usize + im_ser.len());

        // Safety- We just created the vec and leaked it to make it 'static!
        let slice: &[u8] = unsafe { InfoMemPtr::new(start, end) }.into();
        let range = unsafe { InfoMemPtr::new(start, end) }.sequential_read(|addr| {
            Ok(unsafe { *(addr as *const u8) })
        });

        let collected_slice = slice.into_iter().copied().collect::<Vec<u8>>();
        // Safety: We have full control over this allocation and know its good.
        let collected_range: Vec<u8> = range.collect::<Vec<u8>>();

        assert_eq!(collected_slice, collected_range);
    }

    #[test]
    fn test_deser_user_payload_deferred() {
        todo!()
    }
}
