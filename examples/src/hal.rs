/*! In-place HAL for abstracting away writing to some sort of output device
and reading from Information Memory.
*/

#[allow(unused_imports)]
use crate::prelude::{hal::*, osal::*, *};

#[cfg(any(target_os = "none", target_os = "unknown"))]
use super::osal::OurCoreWrite;

cfg_if! {
    if #[cfg(target_arch = "avr")] {
        pub fn mk_iterator<R>(r: R) -> impl Iterator<Item = u8>
        where R: IntoIterator<Item = usize>
        {
            r.into_iter().map(|addr| {
                while EECR::is_set(EECR::EEPE) {}
                EEAR::write(addr as u16);
                EECR::set(EECR::EERE);

                EEDR::read()
            })
        }

        pub fn deserialize_infomem<'buf>(r: Range<usize>, buf: &'buf mut [u8]) -> postcard::Result<InfoMem<'buf>>
        {
            from_seq_magic(r.into_iter().map(|addr| {
                while EECR::is_set(EECR::EEPE) {}
                EEAR::write(addr as u16);
                EECR::set(EECR::EERE);

                Ok(EEDR::read())
            }), buf)
        }
    } else {
        pub fn mk_iterator<R>(r: R) -> impl Iterator<Item = u8>
        where R: IntoIterator<Item = &'static u8>
        {
            r.into_iter().copied()
        }

        pub fn deserialize_infomem<'a, R>(r: R, _buf: &mut [u8]) -> postcard::Result<InfoMem<'a>>
        where R: Into<&'static [u8]>
        {
            from_bytes_magic(r.into())
        }
    }
}

cfg_if! {
    if #[cfg(feature = "ruduino")] {
        pub struct Serial(());

        impl Serial {
            pub fn new() -> Self {
                serial::Serial::new(UBRR)
                    .character_size(serial::CharacterSize::EightBits)
                    .mode(serial::Mode::Asynchronous)
                    .parity(serial::Parity::Disabled)
                    .stop_bits(serial::StopBits::OneBit)
                    .configure();

                port::B5::set_output();

                Self(())
            }
        }

        impl ehal::serial::Write for Serial {
            fn write(&mut self, buffer: &[u8]) -> Result<(), Self::Error> {
                for b in buffer {
                    serial::transmit(*b)
                }

                Ok(())
            }

            fn flush(&mut self) -> Result<(), Self::Error> {
                Ok(())
            }
        }

        impl ehal::serial::ErrorType for Serial {
            type Error = Infallible;
        }

        pub fn mk_writer() -> impl OurCoreWrite<Error = Infallible> {
            Serial::new()
        }
    }
}
