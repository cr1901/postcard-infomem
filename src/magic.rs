pub use ser::to_slice_magic;

#[cfg(feature = "alloc")]
pub use ser::to_allocvec_magic;

#[cfg(feature = "std")]
pub use ser::to_allocvec_magic as to_stdvec_magic;

pub mod ser {
    use core::ops::IndexMut;
    use crate::*;
    use postcard::ser_flavors::{Flavor, Slice};
    use postcard::{Error, Result, serialize_with_flavor};

    #[cfg(feature = "alloc")]
    use postcard::ser_flavors::AllocVec;

    pub fn to_slice_magic<'a>(value: &InfoMem, buf: &'a mut [u8]) -> Result<&'a mut [u8]> {
        let magic = Magic::try_new(Slice::new(buf))?;
        serialize_with_flavor(&value, magic)
    }

    #[cfg(feature = "alloc")]
    pub fn to_allocvec_magic(value: &InfoMem) -> Result<Vec<u8>> {
        let magic = Magic::try_new(AllocVec::default())?;
        serialize_with_flavor(&value, magic)
    }

    pub struct Magic<B>(B)
    where
        B: Flavor + IndexMut<usize, Output = u8>;

    impl<B> Magic<B>
    where
        B: Flavor + IndexMut<usize, Output = u8>,
    {
        pub fn try_new(mut flav: B) -> Result<Self> {
            flav.try_push(b'P')
                .map_err(|_| Error::SerializeBufferFull)?;
            flav.try_push(b'I')
                .map_err(|_| Error::SerializeBufferFull)?;
            flav.try_push(b'M')
                .map_err(|_| Error::SerializeBufferFull)?;
            // Don't try to serialize as UTF-8 string.
            flav.try_push(0x80)
                .map_err(|_| Error::SerializeBufferFull)?;
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
    use crate::*;
    use core::marker::PhantomData;

    use postcard::de_flavors::{Flavor, Slice};
    use postcard::Deserializer;
    use postcard::Result;

    pub fn from_bytes_magic(s: &[u8]) -> Result<InfoMem> {
        let mut de_magic = Deserializer::from_flavor(de::Magic::new(Slice::new(s)));
        InfoMem::deserialize(&mut de_magic)
    }

    #[derive(PartialEq)]
    enum State {
        SawNone,
        SawP,
        SawI,
        SawM,
        Idle,
    }

    pub struct Magic<'de, B>
    where
        B: Flavor<'de>,
    {
        flav: B,
        state: State,
        _phantom: PhantomData<&'de [u8]>,
    }

    impl<'de, B> Magic<'de, B>
    where
        B: Flavor<'de>,
    {
        pub fn new(flav: B) -> Self {
            Self {
                flav,
                state: State::SawNone,
                _phantom: PhantomData,
            }
        }

        fn parse_magic(&mut self) -> Result<()> {
            while self.state != State::Idle {
                let byte = self.flav.pop()?;

                match self.state {
                    State::Idle => {},
                    State::SawNone if byte == b'P' => self.state = State::SawP,
                    State::SawP if byte == b'I' => self.state = State::SawI,
                    State::SawI if byte == b'M' => self.state = State::SawM,
                    State::SawM if byte == 0x80 => self.state = State::Idle,
                    _ if byte == b'P' => self.state = State::SawP,
                    _ => self.state = State::SawNone,
                }
            }

            Ok(())
        }
    }

    impl<'de, B> Flavor<'de> for Magic<'de, B>
    where
        B: Flavor<'de>,
    {
        type Remainder = B::Remainder;
        type Source = B::Source;

        fn pop(&mut self) -> Result<u8> {
            self.parse_magic()?;
            self.flav.pop()
        }

        fn try_take_n(&mut self, ct: usize) -> Result<&'de [u8]> {
            self.parse_magic()?;
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
    use crate::{InfoMem, to_stdvec_magic};
    use postcard::Error;

    extern crate std;
    use std::{print, vec};

    #[test]
    fn test_magic_ser() {
        let im = InfoMem::default();

        let ser = to_stdvec_magic(&im).unwrap();
        ser.iter().for_each(|b| {
            print!("{:02X} ", b);
        });

        assert_eq!(&ser[0..4], &[b'P', b'I', b'M', 0x80]);
    }

    #[test]
    fn test_magic_deser() {
        let im = InfoMem::default();

        let ser = to_stdvec_magic(&im).unwrap();
        let de = from_bytes_magic(&ser).unwrap();

        assert_eq!(im, de);
    }

    #[test]
    fn test_magic_deser_with_data_before() {
        let im = InfoMem::default();
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
        let im = InfoMem::default();
        let mut all_data = vec![b'P', b'I'];

        let ser = to_stdvec_magic(&im).unwrap();
        all_data.extend(ser);
        all_data.iter().for_each(|b| {
            print!("{:02X} ", b);
        });

        let de = from_bytes_magic(&all_data).unwrap();

        assert_eq!(im, de);
    }


    #[test]
    fn test_magic_ok_header_bad_data() {
        let bad_data = [b'P', b'I', b'M', 0x80, 0x00];

        let err = from_bytes_magic(&bad_data).unwrap_err();

        assert_eq!(err, Error::SerdeDeCustom);
    }

    #[test]
    fn test_magic_bad_header_bad_data() {
        // Replace 0x00 with 0x80 for a legal header.
        // Replace '/' with '.' for a legal semver.
        let bad_data = [b'P', b'I', b'M', 0x00, 0x0a, b'0', b'.', b'1', b'/', b'0', b'-', b't', b'e', b's', b't'];

        let err = from_bytes_magic(&bad_data).unwrap_err();

        assert_eq!(err, Error::DeserializeUnexpectedEnd);
    }
}