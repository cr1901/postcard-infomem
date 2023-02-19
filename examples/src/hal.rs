/*! In-place HAL for abstracting away writing to some sort of output device.

Note that [`embedded-hal::serial::Write`] impls [`core::fmt::Write`]
*/

use core::{fmt::Write, convert::Infallible};

use cfg_if::cfg_if;
use postcard_infomem::{ReadSingle, ReadSingleError};

use crate::prelude::*;

cfg_if!{
    if #[cfg(feature = "ruduino")] {
        pub struct EepromReader(Range<usize>);

        impl ReadSingle for EepromReader {
            fn read_single(&mut self) -> Result<u8, ReadSingleError> {
                while EECR::is_set(EECR::EEPE) {}
                EEAR::write(self.0.next().ok_or(ReadSingleError)? as u16);
                EECR::set(EECR::EERE);

                Ok(EEDR::read())
            }
        }

        // Bad... why?
        // impl ReadSingle for EepromReader {
        //     fn read_single(&mut self) -> Result<u8, ReadSingleError> {
        //         let data = self.next().ok_or(ReadSingleError)?;
        //         Ok(data.unwrap())
        //     }
        // }

        impl Iterator for EepromReader {
            type Item = Result<u8, Infallible>;

            fn next(&mut self) -> Option<Self::Item> {
                while EECR::is_set(EECR::EEPE) {}
                EEAR::write(self.0.next()? as u16);
                EECR::set(EECR::EERE);

                Some(Ok(EEDR::read()))
            }
        }

        pub fn output_init() {
            serial::Serial::new(UBRR)
                .character_size(serial::CharacterSize::EightBits)
                .mode(serial::Mode::Asynchronous)
                .parity(serial::Parity::Disabled)
                .stop_bits(serial::StopBits::OneBit)
                .configure();

            port::B5::set_output();
        }

        pub fn mk_reader(infomem: Range<usize>) -> impl ReadSingle + Iterator<Item = Result<u8, Infallible>> {
            EepromReader(infomem) 
        }

        pub fn write(data: &str) {
            for d in data.as_bytes() {
                serial::transmit(*d);
            }
        }

        pub fn write_hex(data: u8) {
            if data < 0x20 || data > 127 {
                serial::transmit(b'\\');
                serial::transmit(b'x');

                let hi = if (data >> 4) & 0xF > 9 {
                    ((data >> 4) & 0xF) + 0x41 - 10
                } else {
                    ((data >> 4) & 0xF) + 0x30
                };

                let lo = if data & 0xF > 9 {
                    (data & 0xF) + 0x41 - 10
                } else {
                    (data & 0xF) + 0x30
                };

                serial::transmit(hi);
                serial::transmit(lo);
            } else {
                serial::transmit(data);
            }
        }
    }
}
