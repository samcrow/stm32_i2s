//! I2S configuration

use core::convert::TryInto;
use core::ops::Range;

use super::format::{DataFormat, FrameFormat};
use super::pac::spi1::i2scfgr::CHLEN_A;
use crate::Polarity;

/// Allowed values for I2S clock division
const DIVISION_RANGE: Range<u16> = 4..512;

/// Configuration for master mode
#[derive(Debug, Clone)]
pub struct MasterConfig<F> {
    /// The clock division between the input clock and bit clock output
    ///
    /// This value is placed into the I2S prescaler register, with the least significant bit in
    /// ODD and the remaining bits in I2SDIV.
    ///
    /// Invariant: This value is in the range `[4, 511]`
    pub(crate) division: u16,
    /// The data format used in memory
    pub(crate) data_format: F,
    /// The frame format used to transmit bits over wires
    pub(crate) frame_format: FrameFormat,
    /// The clock polarity
    pub(crate) polarity: Polarity,
    /// Enable master clock output (256 times the frequency of the word select output)
    pub(crate) master_clock: bool,
}

impl<F> MasterConfig<F>
where
    F: DataFormat,
{
    /// Creates a configuration with a manually specified division from the input clock to the
    /// output bit clock
    ///
    /// # Panics
    ///
    /// This function panics if division is outside the range `[4, 511]`.
    pub fn with_division(
        division: u16,
        data_format: F,
        frame_format: FrameFormat,
        polarity: Polarity,
        master_clock: bool,
    ) -> Self {
        if !DIVISION_RANGE.contains(&division) {
            panic!(
                "I2S clock division {} outside allowed range {}..{}",
                division, DIVISION_RANGE.start, DIVISION_RANGE.end
            );
        }
        MasterConfig {
            division,
            data_format,
            frame_format,
            polarity,
            master_clock,
        }
    }

    /// Creates a configuration with automatic division based on the input clock frequency and
    /// desired sample rate
    ///
    /// frequency_in: The frequency of the I2S clock entering the I2S peripheral, in hertz
    ///
    /// sample_rate: The desired audio sample rate (for each channel) in samples per second
    ///
    /// # Panics
    ///
    /// This function panics if the calculated division is outside the range `[4, 511]`.
    pub fn with_sample_rate(
        frequency_in: u32,
        sample_rate: u32,
        data_format: F,
        frame_format: FrameFormat,
        polarity: Polarity,
        master_clock: bool,
    ) -> Self {
        let bits_per_sample: u32 = match F::CHLEN {
            CHLEN_A::SIXTEENBIT => 16,
            CHLEN_A::THIRTYTWOBIT => 32,
        };
        // Extra division when master clock output is enabled
        let master_clock_division: u32 = if master_clock {
            match F::CHLEN {
                CHLEN_A::SIXTEENBIT => 8,
                CHLEN_A::THIRTYTWOBIT => 4,
            }
        } else {
            1
        };

        // Calculate division based on input frequency and sample rate
        // Calculate actual bit rate
        let bit_rate = sample_rate * 2 * bits_per_sample;
        // sample_rate = frequency_in / ((bits_per_sample * 2) * ((2 * I2SDIV) + ODD) * master_clock_division))
        // substitute division = (2 * I2SDIV) + ODD
        // sample_rate = frequency_in / ((bits_per_sample * 2) * division * master_clock_division)
        // (bits_per_sample * 2) * division * master_clock_division = frequency_in / sample_rate
        // division = frequency_in / (sample_rate * bits_per_sample * 2 * master_clock_division)
        let division = frequency_in / (bit_rate * master_clock_division);

        // Division +/- 1 may result in a more accurate sample rate. Test the three options.
        let division_options: [u32; 3] = [
            division.saturating_sub(1),
            division,
            division.saturating_add(1),
        ];
        let best_division: u32 = division_options
            .iter()
            .cloned()
            .filter(is_valid_division)
            .min_by_key(|possible_division| {
                // Calculate the real sample rate
                let real_sample_rate = frequency_in
                    / (bits_per_sample * 2 * *possible_division * master_clock_division);
                i32::abs((real_sample_rate as i32) - (sample_rate as i32))
            })
            .expect("Couldn't find a valid I2S division value");

        Self::with_division(
            best_division.try_into().unwrap(),
            data_format,
            frame_format,
            polarity,
            master_clock,
        )
    }
}

/// Returns true if the provided value is in the allowed range of division values
fn is_valid_division(division: &u32) -> bool {
    *division >= u32::from(DIVISION_RANGE.start) && *division < u32::from(DIVISION_RANGE.end)
}

/// Configuration for slave mode
#[derive(Debug, Clone)]
pub struct SlaveConfig<F> {
    /// The data format used in memory
    pub(crate) data_format: F,
    /// The frame format used to transmit bits over wires
    pub(crate) frame_format: FrameFormat,
    /// The clock polarity
    pub(crate) polarity: Polarity,
}

impl<F> SlaveConfig<F> {
    pub fn new(data_format: F, frame_format: FrameFormat, polarity: Polarity) -> Self {
        SlaveConfig {
            data_format,
            frame_format,
            polarity,
        }
    }
}
