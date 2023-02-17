/*! Module implementing a [`postcard`] serialization and deserialization
[flavor](postcard#flavors) flavor for prepending/removing a header.
*/

pub use de::from_bytes_magic;
pub use ser::to_slice_magic;

#[cfg(feature = "alloc")]
pub use ser::to_allocvec_magic;

#[cfg(feature = "std")]
pub use ser::to_allocvec_magic as to_stdvec_magic;

pub mod ser {
    /*! Serialization methods and traits for serializing [`InfoMem`] to the
    [`postcard`] wire format.
    */

    use crate::*;
    use core::ops::IndexMut;
    use postcard::ser_flavors::{Flavor, Slice};
    use postcard::{serialize_with_flavor, Result};
    use serde::Serialize;

    #[cfg(feature = "alloc")]
    use postcard::ser_flavors::AllocVec;

    /** Serialize [`InfoMem`] into a [`slice`] with a magic constant header.

    This function is analogous to [`postcard::to_slice`]. */
    pub fn to_slice_magic<'a, T>(value: &InfoMem<T>, buf: &'a mut [u8]) -> Result<&'a mut [u8]>
    where
        T: sealed::Sealed + Serialize,
    {
        let magic = Magic::try_new(Slice::new(buf))?;
        serialize_with_flavor(&value, magic)
    }

    #[cfg(feature = "alloc")]
    /** Serialize [`InfoMem`] into a [`Vec`] with a magic constant header.

    This function is analogous to [`postcard::to_allocvec`]. */
    pub fn to_allocvec_magic<T>(value: &InfoMem<T>) -> Result<Vec<u8>>
    where
        T: sealed::Sealed + Serialize,
    {
        let magic = Magic::try_new(AllocVec::default())?;
        serialize_with_flavor(&value, magic)
    }

    /** A [`postcard`] [flavor](postcard#flavors) for serializing to the
    Postcard wire format with a header.

    The header contains the characters "PIM\x80". This is intended to be the
    top-most serialization flavor; after adding a header, this flavor defers
    to the inner flavor for processing. */
    pub struct Magic<B>(B)
    where
        B: Flavor + IndexMut<usize, Output = u8>;

    impl<B> Magic<B>
    where
        B: Flavor + IndexMut<usize, Output = u8>,
    {
        /**
        Attempt to combine a [`postcard`] [flavor](postcard#flavors) with
        the [`Magic`] serializer to add a magic header.

        # Arguments

        * `flav`: A [`postcard`] [flavor](postcard#flavors), probably a
        [`Slice`] or [`AllocVec`].

        # Errors

        Returns a [`postcard::Error`] from the underlying flavor `B`, if
        adding a header fails.
        */
        pub fn try_new(mut flav: B) -> Result<Self> {
            // End with 0x80 to avoid the temptation to serialize as UTF-8 string.
            flav.try_extend(&[b'P', b'I', b'M', 0x80])?;
            Ok(Self(flav))
        }
    }

    impl<B> Flavor for Magic<B>
    where
        B: Flavor + IndexMut<usize, Output = u8>,
    {
        type Output = <B as Flavor>::Output;

        fn try_push(&mut self, data: u8) -> Result<()> {
            self.0.try_push(data)
        }

        fn finalize(self) -> Result<Self::Output> {
            self.0.finalize()
        }
    }
}

pub mod de {
    /*! Deserialization methods and traits for deserializing [`InfoMem`] from
    the [`postcard`] wire format.
    */

    use crate::*;
    use core::marker::PhantomData;

    use postcard::de_flavors::{Flavor, Slice};
    use postcard::Deserializer;
    use postcard::Result;
    use serde::Deserialize;

    /** Deserialize [`InfoMem`] into a `T`, given a [`slice`] containing
    [`postcard`]-serialized `u8`s preceded by a magic constant header.

    This function is analogous to [`postcard::from_bytes`]. It is intended
    to be used to linearly scan for a serialized [`InfoMem`] `struct` inside a
    "bag of bytes" when the start offset of the `InfoMem` `struct` is not known.

    If the `&[u8]` to be deserialized is _known_ to start with an [`InfoMem`]
    `struct` _without a header_, use [`postcard::from_bytes`]. */
    pub fn from_bytes_magic<'de, T>(s: &'de [u8]) -> Result<InfoMem<T>>
    where
        T: sealed::Sealed + Deserialize<'de>,
    {
        let mut de_magic = Deserializer::from_flavor(de::Magic::try_new(Slice::new(s))?);
        InfoMem::deserialize(&mut de_magic)
    }

    #[derive(PartialEq)]
    /** A state machine [`enum`] for decoding the magic header. */
    enum State {
        /** Start state, or return state if any unexpected character _except 'P'_
        was seen before the entire header was parsed. */
        SawNone,
        /// Saw a 'P', looking for 'I', or wasn't expecting 'P', but found one.
        SawP,
        /// Saw an 'I', looking for 'M'.
        SawI,
        /// Saw an 'M', looking for 0x80.
        SawM,
        /// Saw 0x80, the entire header seen, nothing to do.
        Idle,
    }

    /** A [`postcard`] [flavor](postcard#flavors) for deserializing from the
    Postcard wire format with a header to an [`InfoMem`].

    The header contains the characters "PIM\x80". This is intended to be the
    top-most deserialization flavor; after removing the header, this flavor
    defers to the inner flavor for processing. */
    pub struct Magic<'de, B>
    where
        B: Flavor<'de>,
    {
        /// Deserialization [flavor](postcard#flavors) that this `struct` queries for data.
        flav: B,
        /// Marker type representing the borrowed buffer for deserialization.
        _phantom: PhantomData<&'de [u8]>,
    }

    impl<'de, B> Magic<'de, B>
    where
        B: Flavor<'de>,
    {
        /**
        Attempt to combine a [`postcard`] [flavor](postcard#flavors) with
        the [`Magic`] deserializer to remove a magic header.

        # Arguments

        * `flav`: A [`postcard`] [flavor](postcard#flavors), probably a
        [`Slice`].

        # Errors

        Returns a [`postcard::Error`] from the underlying flavor `B`, if
        finding a header fails.
        */
        pub fn try_new(mut flav: B) -> Result<Self> {
            let mut state = State::SawNone;

            while state != State::Idle {
                let byte = flav.pop()?;

                match state {
                    State::Idle => {}
                    State::SawNone if byte == b'P' => state = State::SawP,
                    State::SawP if byte == b'I' => state = State::SawI,
                    State::SawI if byte == b'M' => state = State::SawM,
                    State::SawM if byte == 0x80 => state = State::Idle,
                    _ if byte == b'P' => state = State::SawP,
                    _ => state = State::SawNone,
                }
            }

            Ok(Self {
                flav,
                _phantom: PhantomData,
            })
        }
    }

    impl<'de, B> Flavor<'de> for Magic<'de, B>
    where
        B: Flavor<'de>,
    {
        type Remainder = B::Remainder;
        type Source = B::Source;

        fn pop(&mut self) -> Result<u8> {
            self.flav.pop()
        }

        fn try_take_n(&mut self, ct: usize) -> Result<&'de [u8]> {
            self.flav.try_take_n(ct)
        }

        fn finalize(self) -> Result<Self::Remainder> {
            self.flav.finalize()
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::de::from_bytes_magic;
    use crate::{to_stdvec_magic, InfoMem};
    use postcard::Error;

    extern crate std;
    use std::{print, vec};

    #[test]
    fn test_magic_ser() {
        let im: InfoMem = InfoMem::default();

        let ser = to_stdvec_magic(&im).unwrap();
        ser.iter().for_each(|b| {
            print!("{:02X} ", b);
        });

        assert_eq!(&ser[0..4], &[b'P', b'I', b'M', 0x80]);
    }

    #[test]
    fn test_magic_deser() {
        let im: InfoMem = InfoMem::default();

        let ser = to_stdvec_magic(&im).unwrap();
        let de = from_bytes_magic(&ser).unwrap();

        assert_eq!(im, de);
    }

    #[test]
    fn test_magic_deser_with_data_before() {
        let im: InfoMem = InfoMem::default();
        let mut all_data = vec![0, 1, 2, 3, 4];

        let ser = to_stdvec_magic(&im).unwrap();
        all_data.extend(ser);
        all_data.iter().for_each(|b| {
            print!("{:02X} ", b);
        });

        let de = from_bytes_magic(&all_data).unwrap();

        assert_eq!(im, de);
    }

    #[test]
    fn test_magic_deser_with_partial_header_before() {
        let im: InfoMem = InfoMem::default();
        let mut all_data = vec![b'P', b'I'];

        let ser = to_stdvec_magic(&im).unwrap();
        all_data.extend(ser);
        all_data.iter().for_each(|b| {
            print!("{:02X} ", b);
        });

        let de = from_bytes_magic::<&[u8]>(&all_data).unwrap();

        assert_eq!(im, de);
    }

    #[test]
    fn test_magic_ok_header_bad_data() {
        let bad_data = [b'P', b'I', b'M', 0x80, 0x00, 0x01, 0x00, 0xff];

        let err = from_bytes_magic::<&[u8]>(&bad_data).unwrap_err();

        assert_eq!(err, Error::DeserializeBadOption);
    }

    #[test]
    fn test_magic_bad_header_bad_data() {
        // Replace 0x00 with 0x80 for a legal header.
        let bad_data = [b'P', b'I', b'M', 0x00, 0x80, 0x00, 0x01, 0x00, 0xff];
        let err = from_bytes_magic::<&[u8]>(&bad_data).unwrap_err();

        assert_eq!(err, Error::DeserializeUnexpectedEnd);
    }
}
