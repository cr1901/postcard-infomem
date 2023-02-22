use core::result::Result as CoreResult;

use super::*;

use postcard::de_flavors::Flavor;
use postcard::{Deserializer, Error, Result};

#[derive(Debug, Clone, Copy)]
pub struct SequentialReadError;

pub trait SequentialRead {
    fn sequential_read(&mut self) -> CoreResult<u8, SequentialReadError>;
}

impl<'de> SequentialRead for &'de [u8] {
    fn sequential_read(&mut self) -> CoreResult<u8, SequentialReadError> {
        let byte = *self.get(0).ok_or(SequentialReadError)?;
        *self = &self[1..];

        Ok(byte)
    }
}

impl<T> sealed::Sealed for T where T: SequentialRead {}

pub struct Seq<R, S> {
    src: R,
    buf: S,
}

impl<R, S> Seq<R, S> {
    pub fn new(src: R, buf: S) -> Self {
        Self { src, buf }
    }
}

impl<'buf, R> Flavor<'buf> for Seq<R, &'buf mut [u8]>
where
    R: SequentialRead + 'buf,
{
    type Remainder = &'buf [u8];
    type Source = &'buf [u8];

    fn pop(&mut self) -> postcard::Result<u8> {
        self.src
            .sequential_read()
            .map_err(|_| Error::DeserializeUnexpectedEnd)
    }

    fn try_take_n(&mut self, ct: usize) -> postcard::Result<&'buf [u8]> {
        if ct > self.buf.len() {
            // this is the wrong error
            return Err(postcard::Error::DeserializeUnexpectedEnd);
        }

        /* Thanks jamesmunns... still no idea why this take is required, but it
        works!

        https://gist.github.com/jamesmunns/de99d22c7dbfd0e47f8ac87e0c1a8872
        */
        let remain = core::mem::take(&mut self.buf);
        let (now, later) = remain.split_at_mut(ct);
        self.buf = later;

        now.iter_mut().try_for_each(|d| {
            // Still the wrong error
            *d = self.pop()?;
            Ok(())
        })?;

        Ok(now)
    }

    fn finalize(self) -> postcard::Result<Self::Remainder> {
        Ok(self.buf)
    }
}

pub fn from_seq_magic<'buf, R, S, T>(src: R, buf: S) -> Result<InfoMem<'buf, T>>
where
    Seq<R, S>: Flavor<'buf>,
    T: sealed::Sealed + Deserialize<'buf>,
    R: SequentialRead + 'buf,
{
    let seq = Seq::new(src, buf);
    let magic = de::Magic::try_new(seq)?;
    let mut de_magic = Deserializer::from_flavor(magic);
    InfoMem::deserialize(&mut de_magic)
}

pub fn from_seq<'buf, R, S, T>(src: R, buf: S) -> Result<InfoMem<'buf, T>>
where
    Seq<R, S>: Flavor<'buf>,
    T: sealed::Sealed + Deserialize<'buf>,
    R: SequentialRead + 'buf,
{
    let seq = Seq::new(src, buf);
    let mut de_seq = Deserializer::from_flavor(seq);
    InfoMem::deserialize(&mut de_seq)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{to_stdvec_magic, InfoMem};

    #[test]
    fn test_seq_deser() {
        let mut im: InfoMem = InfoMem::default();
        im.user = Some(b"test data");

        let mut buf = [0; 127];
        let ser = to_stdvec_magic(&im).unwrap();
        let im_de = from_seq_magic(&*ser, &mut buf).unwrap();

        assert_eq!(im, im_de);
        assert_eq!(&buf[0..9], b"test data");
    }

    #[test]
    fn test_seq_deser_no_room() {
        let mut im: InfoMem = InfoMem::default();
        im.user = Some(b"test data");

        let mut buf = [0; 5];
        let ser = to_stdvec_magic(&im).unwrap();
        let err = from_seq_magic::<_, _, &[u8]>(&*ser, &mut buf).unwrap_err();

        assert_eq!(err, Error::DeserializeUnexpectedEnd);
    }

}
