use core::iter;
use core::ops::Range;
use core::result::Result as CoreResult;

use super::*;

use postcard::de_flavors::Flavor;
use postcard::{Deserializer, Error, Result};

use serde::{self, Deserialize};

#[derive(Debug, Clone, Copy)]
pub struct SequentialReadError;

/* FIXME: Not correct; the Range should start at "the position the deserializer
is at before we started deserializing. For deserializing via postcard, deriving
Deserialize for Range<usize> will deserialize two varints." */
#[derive(Debug, Clone, Deserialize)]
pub(super) struct Deferred(pub Range<usize>);

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
    type Remainder = &'buf [u8];
    type Source = &'buf [u8];

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
        Ok(self.buf)
    }
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
    use core::ptr::addr_of;

    fn seq_vec(
        im_vec: Vec<u8>,
    ) -> iter::Map<Range<usize>, impl FnMut(usize) -> CoreResult<u8, SequentialReadError>> {
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
    #[should_panic]
    #[allow(unreachable_code)]
    fn test_deser_user_payload_deferred() {
        unimplemented!();

        let mut im: InfoMem = InfoMem::default();
        im.user = Some(b"test data");

        let mut buf = [0; 127];
        let ser = to_stdvec_magic(&im).unwrap();
        let ser_addr = addr_of!(ser) as usize;
        
        // FIXME: Not correct... we need a Deferred(Range<usize>) struct
        // which contains information on which address a Seq<> deserialization
        // is at. A naive deserialization of b"test data" into a range gives a range
        // of 9..116, i.e. the length of the slice and the first byte of the slice.
        //
        // Unfortunately, the Deserializer trait doesn't provide a way 
        // (even unsafe!) to get this information so that I could it away
        // into a Deferred(Range<usize>).
        let im_de: InfoMem<Deferred> = from_seq_magic(seq_vec(ser), &mut buf).unwrap();
        
        assert!(im_de.user.is_some());

        println!("{:?}", im_de.user.clone().unwrap());

        let user_seq = im_de.user.unwrap().0.map(|addr| {
            // Safety- 'static/derived from leaked vector.
            Ok(unsafe { *((ser_addr + addr) as *const u8) })
        });

        assert_eq!(buf, [0; 127]);
        
        let user_data: &[u8] = from_seq(user_seq, &mut buf).unwrap();
        assert_eq!(user_data, b"test data");
        assert_eq!(&buf[0..9], b"test data");
    }
}
