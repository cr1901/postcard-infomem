use core::iter;
use core::ops::Range;
use core::result::Result as CoreResult;

use super::*;

use postcard::de_flavors::Flavor;
use postcard::{Deserializer, Error, Result};

use serde::{self, Deserialize};

#[derive(Debug, Clone, Copy)]
pub struct SequentialReadError;

#[derive(Debug, Clone, Deserialize)]
#[repr(transparent)]
pub struct Deferred(usize);

pub struct Seq<R, S> {
    src: R,
    buf: S,
}

impl<R, S> Seq<R, S> {
    pub fn new(src: R, buf: S) -> Self {
        Self { src, buf }
    }
}

impl<'buf, Idx, F> Flavor<'buf> for Seq<iter::Map<Range<Idx>, F>, &'buf mut [u8]>
where
    Idx: 'buf,
    iter::Map<Range<Idx>, F>: Iterator<Item = CoreResult<u8, SequentialReadError>>,
    F: FnMut(Idx) -> CoreResult<u8, SequentialReadError> + 'buf
{
    type Remainder = iter::Map<Range<Idx>, F>;
    type Source = iter::Map<Range<Idx>, F>;

    fn pop(&mut self) -> postcard::Result<u8> {
        self.src
            // Error when range ended early.
            .next().ok_or(Error::DeserializeUnexpectedEnd)?
            // Error for when range didn't end early.
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
        Ok(self.src)
    }
}

fn take_from_seq_magic<'buf, Idx, F, R, S, T>(src: R, buf: S) -> Result<(InfoMem<'buf, T>, iter::Map<Range<Idx>, F>)>
where
    Seq<R, S>: Flavor<'buf, Remainder = std::iter::Map<std::ops::Range<Idx>, F>>,
    T: sealed::Sealed + Deserialize<'buf>,
    F: FnMut(Idx) -> CoreResult<u8, SequentialReadError> + 'buf
{
    let seq = Seq::new(src, buf);
    let magic = de::Magic::try_new(seq)?;
    let mut de_magic = Deserializer::from_flavor(magic);
    let im = InfoMem::deserialize(&mut de_magic)?;
    let rest = de_magic.finalize()?;

    Ok((im, rest))
}

pub fn from_seq_magic_deferred<'buf, Idx, F, R, S>(src: R, buf: S) -> Result<(InfoMem<'buf, Deferred>, iter::Map<Range<Idx>, F>)>
where
    Seq<R, S>: Flavor<'buf, Remainder = std::iter::Map<std::ops::Range<Idx>, F>>,
    F: FnMut(Idx) -> CoreResult<u8, SequentialReadError> + 'buf
{
    take_from_seq_magic(src, buf)
}

pub fn from_seq_magic<'buf, R, S, T>(src: R, buf: S) -> Result<InfoMem<'buf, T>>
where
    Seq<R, S>: Flavor<'buf>,
    T: sealed::Sealed + Deserialize<'buf>,
{
    let seq = Seq::new(src, buf);
    let magic = de::Magic::try_new(seq)?;
    let mut de_magic = Deserializer::from_flavor(magic);
    InfoMem::deserialize(&mut de_magic)
}

pub fn from_seq<'buf, R, S, T>(src: R, buf: S) -> Result<T>
where
    Seq<R, S>: Flavor<'buf>,
    T: Deserialize<'buf>,
{
    let seq = Seq::new(src, buf);
    let mut de_seq = Deserializer::from_flavor(seq);
    T::deserialize(&mut de_seq)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{to_stdvec_magic, InfoMem};
    use postcard::to_stdvec;

    fn seq_vec(
        im_vec: Vec<u8>,
    ) -> iter::Map<Range<usize>, impl FnMut(usize) -> CoreResult<u8, SequentialReadError> + Clone> {
        let im_slice = im_vec.leak();

        (im_slice.as_ptr() as usize..im_slice.as_ptr() as usize + im_slice.len())
            .into_iter()
            .map(|addr| {
                // Safety- 'static.
                Ok(unsafe { *(addr as *const u8) })
            })
    }

    #[test]
    fn test_seq_deser() {
        let mut im: InfoMem = InfoMem::default();
        im.user = Some(b"test data");

        let mut buf = [0; 127];
        let ser = to_stdvec_magic(&im).unwrap();
        let im_de = from_seq_magic(seq_vec(ser), &mut buf).unwrap();

        assert_eq!(im, im_de);
        assert_eq!(&buf[0..9], b"test data");
    }

    #[test]
    fn test_seq_deser_no_room() {
        let mut im: InfoMem = InfoMem::default();
        im.user = Some(b"test data");

        let mut buf = [0; 5];
        let ser = to_stdvec_magic(&im).unwrap();
        let err = from_seq_magic::<_, _, &[u8]>(seq_vec(ser), &mut buf).unwrap_err();

        assert_eq!(err, Error::DeserializeUnexpectedEnd);
    }

    #[test]
    fn test_range_sequential_read_slice_equiv() {
        let im: InfoMem = InfoMem::default();
        let ser = to_stdvec_magic(&im).unwrap();

        let seq_reader = seq_vec(ser.clone());
        let collected_range: Vec<u8> = seq_reader.collect::<CoreResult<_, _>>().unwrap();

        assert_eq!(ser, collected_range);
    }

    #[test]
    fn test_deser_user_payload_deferred() {
        let mut im: InfoMem = InfoMem::default();
        im.app.name = Some(InfoStr::Borrowed("test_deser_user_payload_deferred"));
        im.user = Some(b"test data");

        let mut buf = [0; 64];
        let ser = to_stdvec_magic(&im).unwrap();

        let (im_de, rest) = from_seq_magic_deferred(seq_vec(ser), &mut buf).unwrap();
        assert!(im_de.user.is_some());
        assert_eq!(&buf[0..32], b"test_deser_user_payload_deferred");
        
        let user_data = rest.collect::<CoreResult<Vec<u8>, SequentialReadError>>().unwrap();
        assert_eq!(user_data, b"test data");
    }

    #[test]
    fn test_deser_user_payload_serialized_deferred() {
        let mut im: InfoMem<Vec<u8>> = InfoMem::default();
        im.app.name = Some(InfoStr::Borrowed("test_deser_user_payload_deferred"));
        im.user = Some(to_stdvec(&(0xff, b"test data".as_ref())).unwrap());

        let mut buf = [0; 64];
        let mut user_buf = [0; 64];
        let ser = to_stdvec_magic(&im).unwrap();

        let (im_de, rest) = from_seq_magic_deferred(seq_vec(ser), &mut buf).unwrap();
        assert!(im_de.user.is_some());
        assert_eq!(&buf[0..32], b"test_deser_user_payload_deferred");

        let user_data: (_, &[u8]) = from_seq(rest, &mut user_buf).unwrap();
        assert_eq!(user_data, (0xff, b"test data".as_ref()));
        assert_eq!(&user_buf[0..9], b"test data");
    }
}
