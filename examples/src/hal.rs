/*! In-place HAL for abstracting away writing to some sort of output device
and reading from Information Memory.
*/

#[allow(unused_imports)]
use crate::prelude::{hal::*, osal::*, *};

#[cfg(any(target_os = "none", target_os = "unknown"))]
use super::osal::OurCoreWrite;

cfg_if! {
    if #[cfg(target_arch = "avr")] {
        /* In practice, we should make sure either:
        1. One thread (probably main) has access to EEPROM.
        2. If multiple threads (probably main and interrupts) need access, they
           are using synchronization. One example of synchronization is "all
           AVR [`Peripherals`](https://docs.rs/avr-device/latest/avr_device/atmega328p/struct.Peripherals.html)
           wrapped in a [`OnceCell`](https://docs.rs/once_cell/latest/once_cell/unsync/struct.OnceCell.html)
           wrapped in a [`critical_section::Mutex](https://docs.rs/critical-section/latest/critical_section/struct.Mutex.html)".

        See: https://blog.japaric.io/brave-new-io/

        It's likely not UB/not a data race, but multiple threads interleaving
        reads/writes is still probably not what you want.
        */
        fn read_eeprom(addr: usize) -> Result<u8, SequentialReadError> {
            while EECR::is_set(EECR::EEPE) {}
            EEAR::write(addr as u16);
            EECR::set(EECR::EERE);

            Ok(EEDR::read())
        }

        pub fn mk_iterator<R>(r: R) -> impl Iterator<Item = u8>
        where R: IntoIterator<Item = usize>
        {
            r.into_iter().map_while(|addr| read_eeprom(addr).ok())
        }

        pub fn deserialize_infomem<'buf, R>(r: R, buf: &'buf mut [u8]) -> postcard::Result<InfoMem<'buf>>
        where R: Into<Range<usize>>
        {
            from_seq_magic(r.into().into_iter().map(read_eeprom), buf)
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
