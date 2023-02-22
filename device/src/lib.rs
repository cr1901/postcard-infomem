/*! Helper crate for [`InfoMem`](../postcard_infomem/struct.InfoMem.html) `struct`s
intended to primarily be used in `no_std` environments (although this does not
preclude using the crate for hosted applications).
*/
#![no_std]

use core::iter::Copied;
use core::slice::from_raw_parts;
use core::{ops, slice};

use postcard_infomem::{ReadSingle, ReadSingleError};
use serde::Deserialize;

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
            pub fn get<T>() -> T
            where
                T: From<$crate::Ptr>,
            {
                // SAFETY: `Ptr` is derived from a static array with known length.
                unsafe { $crate::Ptr::new(INFOMEM.as_ptr(), INFOMEM.len()).into() }
            }
        }
    };
}

/** Pointer to an infomem struct that is address-space aware.

The [`Ptr`] `struct` is a wrapper over a pointer that can be used as
a portable type to access the `INFOMEM` `static` on targets with one or
multiple address spaces.

It is meant to be immediately converted to another, more ergonomic type for
your environment. */
pub struct Ptr(*const u8, usize);

impl Ptr {
    /** Create a [`Ptr`] to an `INFOMEM` `static`.

    ## Safety

    Due to publicly-available [`From`] impls, the pointer needs to
    point to a valid memory block that's _not_ currently mutably borrowed.
    Otherwise, Undefined Behavior may occur when [`Ptr`] is converted.

    [`include_postcard_infomem`] takes care of safely creating a [`Ptr`] for you.
    */
    pub unsafe fn new(ptr: *const u8, len: usize) -> Self {
        Ptr(ptr, len)
    }
}

#[cfg(not(target_arch = "avr"))]
impl<'a> From<Ptr> for Slice<'a> {
    fn from(value: Ptr) -> Self {
        // SAFETY: You have already opted into `unsafe` to create
        // a `Ptr`, and are upholding `Ptr`s invariants before
        // doing the conversion.
        Self(unsafe { from_raw_parts(value.0, value.1) })
    }
}

/** Newtype over a `'static` reference to `INFOMEM`.

[`Slice`] is returned by `infomem::get` on most architectures, where
the `INFOMEM` static can be accessed safely. A newtype is used
instead of returning the `&'static [u8]` directly so that conditionally
compiled code operating on [`Slice`] can have the same structure
regardless of whether the architecture uses [`Slice`] or [`Range`]
to access `INFOMEM`.

In particular, [`Slice`] and adapters that consume [`Range`] will
create iterators over `u8`, while `&'static [u8]` creates an iterator
over `&u8`. */
#[derive(Clone, Copy, Deserialize)]
#[cfg(not(target_arch = "avr"))]
pub struct Slice<'a>(&'a [u8]);

#[cfg(not(target_arch = "avr"))]
impl<'a> From<Slice<'a>> for &'a [u8] {
    fn from(value: Slice<'a>) -> Self {
        value.0
    }
}

#[cfg(not(target_arch = "avr"))]
impl<'a> IntoIterator for Slice<'a> {
    type Item = u8;
    type IntoIter = Copied<slice::Iter<'a, u8>>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter().copied()
    }
}

#[cfg(not(target_arch = "avr"))]
impl<'a> ReadSingle for Slice<'a> {
    fn read_single(&mut self) -> Result<u8, ReadSingleError> {
        self.0.read_single()
    }
}

/** Newtype over a `Range` of `INFOMEM` addresses.

When a reference to an `INFOMEM` `struct` cannot be safely returned, such as
on Harvard architectures, one can iterate over the range of addresses used
for the `INFOMEM` `struct` instead. The addresses are represented as [`usize`]s.

This type is intended to be used as part of an [`Iterator`] adapter, to access
the `INFOMEM` in a platform-specific manner. See [`read_single`]. */
#[derive(Clone)]
pub struct Range(ops::Range<usize>);

impl Range {
    /** Create an adapter from [`Range`] to `INFOMEM` `u8` that implements
    [`ReadSingle`] and [`Iterator`].
    
    [`Range`] by itself returns [`usize`]s that represent addresses. The user
    takes addresses supplied by [`Range`] to access `INFOMEM` in a platform
    specific manner. For instance, the following two loops which print the
    contents of `INFOMEM` as [`UpperHex`] are equivalent:

    ```ignore
    for data in infomem::get::<Range>().read_single(|addr| {
        // SAFTEY: we have to opt into unsafety to create a Range,
        // a range provided by include_postcard_infomem macro will be
        // safe to dereference over all `usize`s passed to this closure.
        Ok(unsafe { *(addr as *const u8) })
    }) {
        write!(w, "{:X}", data).unwrap();
    }
    ```

    ```ignore
    for data in infomem::get::<Slice>() {
        write!(w, "{:X}", data).unwrap();
    }
    ``` */
    pub fn read_single<F>(self, f: F) -> impl ReadSingle + Iterator<Item = u8> + Clone where F: FnMut(usize) -> Result<u8, ReadSingleError> + Clone {
        RangeReadSingle {
            range: self,
            f
        }
    }
}

impl From<Ptr> for Range {
    fn from(value: Ptr) -> Self {
        Self(ops::Range {
            start: value.0 as usize,
            end: value.0 as usize + value.1,
        })
    }
}

impl Iterator for Range {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

/* `struct` which maps [`usize`] addresses to `INFOMEM` contents outside of
the data address space.

This `struct` is a convenience to avoid the need to create unique types
for each platform that implement the [`ReadSingle`] trait. It also implements
[`Iterator`] for parity with [`Slice`]; errors are mapped to [`None`] when
used as an iterator.
*/
#[derive(Clone)]
pub struct RangeReadSingle<F> {
    range: Range,
    f: F
}

impl<F> ReadSingle for RangeReadSingle<F> where F: FnMut(usize) -> Result<u8, ReadSingleError> {
    fn read_single(&mut self) -> Result<u8, ReadSingleError> {
        let addr = self.range.next().ok_or(ReadSingleError)?;
        (self.f)(addr)
    }
}

impl<F> Iterator for RangeReadSingle<F> where F: FnMut(usize) -> Result<u8, ReadSingleError> {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        let addr = self.range.next()?;
        (self.f)(addr).ok()
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
    fn test_range_read_single_slice_equiv() {
        let im_ser = to_allocvec_magic(&InfoMem::<&[u8]>::default()).unwrap().leak();

        // Safety- We just created the vec and leaked it to make it 'static!
        let slice: Slice = unsafe { Ptr::new(im_ser.as_ptr(), im_ser.len()) }.into();
        let range: Range = unsafe { Ptr::new(im_ser.as_ptr(), im_ser.len()) }.into();

        let collected_slice = slice.into_iter().collect::<Vec<u8>>();

        // Safety: We have full control over this allocation and know its good.
        let collected_range: Vec<u8> = range.read_single(|addr| {
            Ok(unsafe { *(addr as *const u8) })
        }).collect::<Vec<u8>>();

        assert_eq!(collected_slice, collected_range);
    }

    #[test]
    fn test_deser_user_payload_deferred() {
        let mut im = InfoMem::<&[u8]>::default();
        let user_payload = &(b"test data" as &[u8], 42);

        let user_tuple_ser = to_allocvec(&user_payload).unwrap().leak();
        im.user = Some(user_tuple_ser);
        let im_ser = to_allocvec_magic(&im).unwrap().leak();

        // Safety- We just created the vec and leaked it to make it 'static!
        let slice: Slice = unsafe { Ptr::new(im_ser.as_ptr(), im_ser.len()) }.into();

        let mut buf = [0; 16];
        let im_ser_payload = from_seq_magic::<_, _, Slice>(slice, &mut buf).unwrap();

        assert_eq!(im_ser_payload.user.unwrap().0, user_tuple_ser);
        assert_eq!(user_payload, &from_bytes::<(&[u8], i32)>(im_ser_payload.user.unwrap().0).unwrap());
    }
}
