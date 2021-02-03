//! 24-bit data format in a 32-bit frame

use super::detail::DataFormatDetail;
use super::{read_two_steps, write_two_steps, DataFormat, DataFormatType, FrameFormat};
use crate::v12x::pac::spi1::i2scfgr::{CHLEN_A, DATLEN_A};
use crate::v12x::pac::spi1::RegisterBlock;

/// 32 bits transferred for each audio sample, represented in memory with 24 bits per sample
///
/// # Receiving
///
/// When receiving, the 8 least significant bits are ignored. Each received sample will be sign-
/// extended to 32 bits.
///
/// ## Example (I2S, MSB justified, or PCM)
///
/// Bits on wire: `1000 1110 1010 1010 0011 0011 xxxx xxxx`
///
/// Received 32-bit sample in memory: `1111 1111 1000 1110 1010 1010 0011 0011` (`0xff8eaa33`)
///
/// ## Example (LSB justified)
///
/// Bits on wire: `xxxx xxxx 1000 1110 1010 1010 0011 0011`
///
/// Received 32-bit sample in memory: `1111 1111 1000 1110 1010 1010 0011 0011` (`0xff8eaa33`)
///
/// # Transmitting
///
/// When transmitting, the 8 most significant bits of each 32-bit sample are ignored. The final
/// 8 bits sent on the wire will all be zero.
///
/// ## Example (I2S, MSB justified, or PCM)
///
/// 32-bit sample in memory: `xxxx xxxx 1000 1110 1010 1010 0011 0011` (8 most significant bits
/// can be anything, other 24 bits are 0x8eaa33)
///
/// Bits on wire: `1000 1110 1010 1010 0011 0011 0000 0000`
///
/// ## Example (LSB justified)
///
/// 32-bit sample in memory: `xxxx xxxx 1000 1110 1010 1010 0011 0011` (8 most significant bits
/// can be anything, other 24 bits are 0x8eaa33)
///
/// Bits on wire: `0000 0000 1000 1110 1010 1010 0011 0011`
///
///
#[derive(Debug, Clone)]
pub struct Data24Frame32;

impl DataFormat for Data24Frame32 {}
impl DataFormatType for Data24Frame32 {
    type Sample = i32;
}
impl DataFormatDetail for Data24Frame32 {
    const DATLEN: DATLEN_A = DATLEN_A::TWENTYFOURBIT;
    const CHLEN: CHLEN_A = CHLEN_A::THIRTYTWOBIT;

    fn read_sample(format: &FrameFormat, registers: &RegisterBlock) -> i32 {
        match format {
            FrameFormat::LsbJustified => build_sample_lsb_justified(read_two_steps(registers)),
            FrameFormat::PhilipsI2s | FrameFormat::MsbJustified | FrameFormat::Pcm(_) => {
                build_sample_msb_justified(read_two_steps(registers))
            }
        }
    }

    fn write_sample(format: &FrameFormat, registers: &RegisterBlock, sample: i32) {
        match format {
            FrameFormat::LsbJustified => {
                write_two_steps(registers, split_sample_lsb_justified(sample));
            }
            FrameFormat::PhilipsI2s | FrameFormat::MsbJustified | FrameFormat::Pcm(_) => {
                write_two_steps(registers, split_sample_msb_justified(sample));
            }
        }
    }
}

/// Builds a sample from two data register reads for I2S, MSB justified, or PCM formats
fn build_sample_msb_justified(values: [u16; 2]) -> i32 {
    // Read 1 has the two middle bytes, read 2 has the least significant byte followed by an unspecified value
    let read1 = values[0];
    let read2 = values[1] & 0xff00;
    let sample = (u32::from(read1) << 8) | (u32::from(read2) >> 8);
    sign_extend_24_to_32(sample) as i32
}

/// Builds a sample from two data register reads for LSB justified format
fn build_sample_lsb_justified(values: [u16; 2]) -> i32 {
    // Read 1 has the most significant bytes, read 2 has the other two bytes
    let read1 = values[0] & 0x00ff;
    let read2 = values[1];
    let sample = (u32::from(read1) << 16) | u32::from(read2);
    sign_extend_24_to_32(sample) as i32
}

/// Sign-extends a 24-bit integer into 32 bits
fn sign_extend_24_to_32(value: u32) -> u32 {
    if ((value >> 23) & 1) == 1 {
        0xff000000 | value
    } else {
        value
    }
}

/// Splits a 32-bit sample into two data register writes for I2S, MSB justified, or PCM formats
fn split_sample_msb_justified(sample: i32) -> [u16; 2] {
    let sample = sample as u32;
    // Write 1 has the two middle bytes, write 2 has the least significant byte followed by 0x00
    let write1 = (sample >> 8) as u16;
    let write2 = ((sample & 0xff) << 8) as u16;
    [write1, write2]
}

/// Splits a 32-bit sample into two data register writes for LSB justified format
fn split_sample_lsb_justified(sample: i32) -> [u16; 2] {
    let sample = sample as u32;
    // Write 1 has 0x00 and the most significant byte, write 2 has the two least significant bytes
    let write1 = ((sample >> 16) & 0xff) as u16;
    let write2 = sample as u16;
    [write1, write2]
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn build_msb() {
        assert_eq!(
            0x003478ae_u32 as i32,
            build_sample_msb_justified([0x3478, 0xae00])
        );
        assert_eq!(
            0xff8eaa33_u32 as i32,
            build_sample_msb_justified([0x8eaa, 0x3300])
        );
    }

    #[test]
    fn build_lsb() {
        assert_eq!(
            0x003478ae_u32 as i32,
            build_sample_lsb_justified([0x0034, 0x78ae])
        );
        assert_eq!(
            0xff8eaa33_u32 as i32,
            build_sample_lsb_justified([0x008e, 0xaa33])
        );
    }

    #[test]
    fn split_msb() {
        assert_eq!(
            [0x3478, 0xae00],
            split_sample_msb_justified(0x003478ae_u32 as i32)
        );
        assert_eq!(
            [0x8eaa, 0x3300],
            split_sample_msb_justified(0xff8eaa33_u32 as i32)
        );
        assert_eq!(
            [0x8eaa, 0x3300],
            split_sample_msb_justified(0x008eaa33_u32 as i32)
        );
    }

    #[test]
    fn split_lsb() {
        assert_eq!(
            [0x0034, 0x78ae],
            split_sample_lsb_justified(0x003478ae_u32 as i32)
        );
        assert_eq!(
            [0x008e, 0xaa33],
            split_sample_lsb_justified(0xff8eaa33_u32 as i32)
        );
        assert_eq!(
            [0x008e, 0xaa33],
            split_sample_lsb_justified(0x008eaa33_u32 as i32)
        );
    }
}
