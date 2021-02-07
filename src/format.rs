//! I2S frame formats

mod data24frame32;

use crate::pac::spi1::i2scfgr::{CHLEN_A, DATLEN_A};
use crate::pac::spi1::RegisterBlock;

pub use self::data24frame32::Data24Frame32;

/// I2S communication frame format standards
#[derive(Debug, Clone)]
pub enum FrameFormat {
    /// Philips I2S
    PhilipsI2s,
    /// Bits justified with the most significant bit aligned to the beginning of the frame,
    /// potentially with unused bits at the end
    MsbJustified,
    /// Bits justified with the least significant bit aligned to the end of the frame,
    /// potentially with unused bits at the beginning
    LsbJustified,
    /// Pulse code modulation
    Pcm(FrameSync),
}

/// PCM frame synchronization modes
#[derive(Debug, Clone)]
pub enum FrameSync {
    /// WS pulses high just before the most significant bit of each sample
    Short,
    /// WS is high for the first 13 bits of each sample
    Long,
}

/// A supported audio sample format
///
/// This trait provides the sample type only.
pub trait DataFormatType {
    /// The type used to represent a sample in memory
    type Sample: Copy;
}

mod detail {
    use super::{DataFormatType, FrameFormat};
    use crate::pac::spi1::i2scfgr::{CHLEN_A, DATLEN_A};
    use crate::pac::spi1::RegisterBlock;
    /// A supported audio format (private implementation details)
    pub trait DataFormatDetail: DataFormatType {
        /// Size of audio samples in memory (DATLEN field of SPI_I2SCFGR)
        const DATLEN: DATLEN_A;
        /// Number of bits used on wire for each sample (CHLEN field of SPI_I2SCFGR)
        const CHLEN: CHLEN_A;
        /// Reads one sample from the data register and returns it as a frame value
        ///
        /// When using a 24-bit or 32-bit data format, this function blocks until the both parts
        /// of the sample have been received.
        fn read_sample(
            format: &FrameFormat,
            registers: &RegisterBlock,
        ) -> <Self as DataFormatType>::Sample;
        /// Writes one frame to the data register as a sample
        ///
        /// When this function is called, the TXE bit of the status register must be set.
        ///
        /// When using a 24-bit or 32-bit data format, this function blocks until the first part
        /// of the sample has been transmitted.
        fn write_sample(
            format: &FrameFormat,
            registers: &RegisterBlock,
            sample: <Self as DataFormatType>::Sample,
        );
    }
}
use self::detail::DataFormatDetail;

/// A supported audio sample format
///
/// This trait inherits from DataFormatType (indirectly) so that the Sample type is public
/// but the other trait items are private.
pub trait DataFormat: DataFormatDetail {}

// Utility functions

/// Writes a sample to the data register
fn write_one_step(registers: &RegisterBlock, value: u16) {
    registers.dr.write(|w| w.dr().bits(value));
}
/// Writes one sample to the data register, waits for the sample to be transmitted,
/// and writes the second sample to the data register
fn write_two_steps(registers: &RegisterBlock, values: [u16; 2]) {
    registers.dr.write(|w| w.dr().bits(values[0]));
    // Wait for the register to become empty again
    while registers.sr.read().txe().is_not_empty() {}
    registers.dr.write(|w| w.dr().bits(values[1]));
}

/// Reads two consecutive samples from the data register, waiting for the second sample to appear
fn read_two_steps(registers: &RegisterBlock) -> [u16; 2] {
    let value1 = registers.dr.read().dr().bits();
    while registers.sr.read().rxne().is_empty() {}
    let value2 = registers.dr.read().dr().bits();
    [value1, value2]
}

/// Reads a sample from the data register
fn read_one_step(registers: &RegisterBlock) -> u16 {
    registers.dr.read().dr().bits()
}

/// 16 bits transferred for each audio sample, represented in memory with 16 bits per sample
#[derive(Debug, Clone)]
pub struct Data16Frame16;

impl DataFormat for Data16Frame16 {}
impl DataFormatType for Data16Frame16 {
    type Sample = i16;
}
impl DataFormatDetail for Data16Frame16 {
    const DATLEN: DATLEN_A = DATLEN_A::SIXTEENBIT;
    const CHLEN: CHLEN_A = CHLEN_A::SIXTEENBIT;

    fn read_sample(_format: &FrameFormat, registers: &RegisterBlock) -> i16 {
        // Just one 16-bit read
        read_one_step(registers) as i16
    }

    fn write_sample(_format: &FrameFormat, registers: &RegisterBlock, sample: i16) {
        // Just one 16-bit write
        write_one_step(registers, sample as u16);
    }
}

/// 32 bits transferred for each audio sample, represented in memory with 16 bits per sample
///
/// When receiving, the 16 least significant bits are ignored. When transmitting, the sample
/// is sent in the 16 most significant bits and the other 16 bits are sent as zeros.
#[derive(Debug, Clone)]
pub struct Data16Frame32;

impl DataFormat for Data16Frame32 {}
impl DataFormatType for Data16Frame32 {
    type Sample = i16;
}
impl DataFormatDetail for Data16Frame32 {
    const DATLEN: DATLEN_A = DATLEN_A::SIXTEENBIT;
    const CHLEN: CHLEN_A = CHLEN_A::THIRTYTWOBIT;

    fn read_sample(_format: &FrameFormat, registers: &RegisterBlock) -> i16 {
        // Just one read
        read_one_step(registers) as i16
    }

    fn write_sample(_format: &FrameFormat, registers: &RegisterBlock, sample: i16) {
        // Just one write
        write_one_step(registers, sample as u16);
    }
}

/// 32 bits in each audio sample, represented in memory with 32 bits per sample
#[derive(Debug, Clone)]
pub struct Data32Frame32;

impl DataFormat for Data32Frame32 {}
impl DataFormatType for Data32Frame32 {
    type Sample = i32;
}
impl DataFormatDetail for Data32Frame32 {
    const DATLEN: DATLEN_A = DATLEN_A::THIRTYTWOBIT;
    const CHLEN: CHLEN_A = CHLEN_A::THIRTYTWOBIT;

    fn read_sample(_format: &FrameFormat, registers: &RegisterBlock) -> i32 {
        // Two reads, most significant half first
        let [msbs, lsbs] = read_two_steps(registers);
        ((u32::from(msbs) << 16) | u32::from(lsbs)) as i32
    }

    fn write_sample(_format: &FrameFormat, registers: &RegisterBlock, sample: i32) {
        // Two writes, most significant half first
        let msbs = ((sample as u32) >> 16) as u16;
        let lsbs = sample as u16;
        write_two_steps(registers, [msbs, lsbs]);
    }
}
