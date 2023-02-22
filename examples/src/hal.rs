/*! In-place HAL for abstracting away writing to some sort of output device
and reading from Information Memory.
*/

#[allow(unused_imports)]
use crate::prelude::{*, hal::*, osal::*};

#[cfg(any(target_os = "none", target_os = "unknown"))]
use super::osal::OurCoreWrite;

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

        pub fn mk_reader(infomem: Range) -> impl SequentialRead + IntoIterator<Item = u8> + Clone {
            infomem.sequential_read(|addr| {
                while EECR::is_set(EECR::EEPE) {}
                EEAR::write(addr as u16);
                EECR::set(EECR::EERE);

                Ok(EEDR::read())
            })
        }
    }
}
