/*! In-place HAL for abstracting away writing to some sort of output device
and reading from Information Memory.
*/

use crate::prelude::*;

pub struct Ascii(u8);

impl From<u8> for Ascii {
    fn from(value: u8) -> Self {
        Self(value)
    }
}

impl fmt::Display for Ascii {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0 < 0x20 || self.0 > 127 {
            write!(f, "\\x{:02X}", self.0)?;
        } else {
            write!(f, "{}", self.0 as char)?;
        }

        Ok(())
    }
}

cfg_if! {
    if #[cfg(any(target_os = "none", target_os="unknown"))] {
        use embedded_hal as ehal;

        pub trait OurCoreWrite: ehal::serial::Write {}
        impl<T> OurCoreWrite for T where T: ehal::serial::Write {}

        impl<Error> fmt::Write for dyn OurCoreWrite<Error = Error> where Error: ehal::serial::Error {
            fn write_str(&mut self, s: &str) -> Result<(), core::fmt::Error> {
                self.write(s.as_bytes()).map_err(|_| core::fmt::Error)
            }
        }
    } else {
        pub fn mk_writer() -> impl io::Write {
            io::stdout()
        }

        pub fn mk_reader(infomem: Slice) -> impl ReadSingle + IntoIterator<Item = u8> + Clone {
            infomem
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

        pub fn mk_reader(infomem: Range) -> impl ReadSingle + IntoIterator<Item = u8> + Clone {
            infomem.read_single(|addr| {
                while EECR::is_set(EECR::EEPE) {}
                EEAR::write(addr as u16);
                EECR::set(EECR::EERE);

                Ok(EEDR::read())
            })
        }
    }
}
