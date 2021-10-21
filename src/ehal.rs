//! Implements embedded-hal I2S traits

use crate::format::DataFormat;
use crate::{Channel, I2s, Instance, ReceiveError, ReceiveMode, TransmitError, TransmitMode};
use ehal1::blocking::i2s::{Read, Write, WriteIter};

impl<I, F> Write<F::Sample> for I2s<I, TransmitMode<F>>
where
    I: Instance,
    F: DataFormat,
{
    type Error = TransmitError;

    /// Writes a sequence of words
    ///
    /// This function returns an error if an underrun occurs because reading a word
    /// from `left_words` or `right_words` took too long. This may happen with high sample rates
    /// and low processor speeds.
    ///
    /// If the lengths of left_words and right_words are not equal, this sends the smaller number
    /// of samples and ignores samples at the end that are found in the left or right only.
    fn try_write<'w>(
        &mut self,
        left_words: &'w [F::Sample],
        right_words: &'w [F::Sample],
    ) -> Result<(), Self::Error> {
        self.try_write_iter(left_words.iter().copied(), right_words.iter().copied())
    }
}

impl<I, F> WriteIter<F::Sample> for I2s<I, TransmitMode<F>>
where
    I: Instance,
    F: DataFormat,
{
    type Error = TransmitError;

    /// Writes a sequence of words
    ///
    /// This function returns an error if an underrun occurs because a word is not available
    /// from `left_words` or `right_words`.
    ///
    /// If the lengths of left_words and right_words are not equal, this sends the smaller number
    /// of samples and ignores samples at the end that are found in the left or right only.
    fn try_write_iter<LW, RW>(&mut self, left_words: LW, right_words: RW) -> Result<(), Self::Error>
    where
        LW: IntoIterator<Item = F::Sample>,
        RW: IntoIterator<Item = F::Sample>,
    {
        // Clear any previous errors
        let _ = self.take_error();
        // Block until the peripheral is ready to transmit, and find out which channel will be first
        let start_channel = self.block_until_ready_to_transmit();
        // Transmit each word
        // If the lengths of left_words and right_words are not equal, this sends the smaller number
        // of samples and ignores samples at the end that are found in the left or right only.
        for (left_word, right_word) in left_words.into_iter().zip(right_words.into_iter()) {
            match start_channel {
                Channel::Left => {
                    self.take_error()?;
                    nb::block!(self.transmit(left_word)).unwrap();
                    self.take_error()?;
                    nb::block!(self.transmit(right_word)).unwrap();
                }
                Channel::Right => {
                    self.take_error()?;
                    nb::block!(self.transmit(right_word)).unwrap();
                    self.take_error()?;
                    nb::block!(self.transmit(left_word)).unwrap();
                }
            }
        }

        Ok(())
    }
}

impl<I, F> Read<F::Sample> for I2s<I, ReceiveMode<F>>
where
    I: Instance,
    F: DataFormat,
{
    type Error = ReceiveError;

    fn try_read<'w>(
        &mut self,
        left_words: &'w mut [F::Sample],
        right_words: &'w mut [F::Sample],
    ) -> Result<(), Self::Error> {
        // Clear any earlier errors
        let _ = self.take_error();

        for (left, right) in left_words.iter_mut().zip(right_words.iter_mut()) {
            self.take_error()?;
            // Read the first sample, which may be on either channel
            {
                let (sample, channel) = nb::block!(self.receive()).unwrap();
                match channel {
                    Channel::Left => *left = sample,
                    Channel::Right => *right = sample,
                }
            }
            self.take_error()?;
            // Read another sample, which should be on the other channel
            {
                let (sample, channel) = nb::block!(self.receive()).unwrap();
                match channel {
                    Channel::Left => *left = sample,
                    Channel::Right => *right = sample,
                }
            }
        }

        Ok(())
    }
}
