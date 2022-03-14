//! This library supports I2S communication for SPI version 1.2 (on STM32F1, STM32F2, STM32F4,
//! STM32L0, and STM32L1 microcontrollers).
//!
//! This library is normally used with a HAL library that provides a type that implements
//! [Instance](crate::Instance). An [I2s](crate::I2s) object can be created around the Instance
//! object and used for I2S.

#![no_std]

extern crate nb;
extern crate vcell;

mod config;
pub mod format;
mod pac;

use core::convert::Infallible;
use core::marker::PhantomData;

pub use self::config::{MasterClock, MasterConfig, SlaveConfig};
pub use self::pac::spi1::RegisterBlock;
use crate::format::{DataFormat, FrameFormat, FrameSync};
use crate::pac::spi1::i2scfgr::I2SCFG_A;

/// Clock polarity
#[derive(Debug, Clone)]
pub enum Polarity {
    /// Clock low when idle
    IdleLow,
    /// Clock high when idle
    IdleHigh,
}

/// The channel associated with a sample
#[derive(Debug, Clone, PartialEq)]
pub enum Channel {
    /// Left channel (word select low)
    Left,
    /// Right channel (word select high)
    Right,
}

/// Events with associated interrupts that can be enabled
pub enum Event {
    /// The transmit data register is empty, and a sample can be written
    TransmitEmtpy,
    /// The receive data register is not empty, and a sample can be read
    ReceiveNotEmpty,
    /// An error has occurred
    Error,
}

/// An SPI peripheral instance that can be used for I2C communication
///
/// This trait is meant to be implemented for a HAL-specific type that represent ownership of
/// the SPI peripheral (and any pins required by it, although that is entirely up to the HAL).
///
/// # Safety
///
/// It is only safe to implement this trait when:
///
/// * The implementing type has ownership of the peripheral, preventing any other accesses to the
///   register block.
/// * `REGISTERS` is a pointer to that peripheral's register block and can be safely accessed for as
///   long as ownership or a borrow of the implementing type is present.
pub unsafe trait Instance {
    /// Pointer to the SPI register block
    const REGISTERS: *mut RegisterBlock;
}

/// A dual I2S instance composed by a SPI in I2S mode and a I2SEXT peripheral
///
/// TODO!
pub unsafe trait DualInstance {
    const REGISTERS: (*mut RegisterBlock, *mut RegisterBlock);
}

/// Interface to an SPI peripheral in I2S mode
///
/// # Basic sequence of operations
///
/// * Create an I2s object
/// * Configure it in the desired mode (master/slave, transmit/receive)
/// * Enable interrupts and DMA (optional)
/// * Enable
/// * Transmit or receive samples
/// * Disable
/// * Deconfigure the I2s, allowing it to be configured again differently
///
/// # Example
///
/// ```no_run
/// # use stm32_i2s::{I2s, Instance, MasterConfig, InitMode, Polarity, MasterClock};
/// # use stm32_i2s::format::{Data16Frame16, FrameFormat};
/// fn use_i2s<I>(i2s: I2s<I, InitMode>) where I: Instance {
///     let config = MasterConfig::with_division(
///         25,
///         Data16Frame16,
///         FrameFormat::PhilipsI2s,
///         Polarity::IdleHigh,
///         MasterClock::Disble,
///     );
///     let mut i2s_configured = i2s.configure_master_receive(config);
///     let mut samples: [i16; 64] = [0; 64];
///     i2s_configured.enable();
///     i2s_configured.receive_blocking(&mut samples);
///     i2s_configured.disable();
/// }
/// ```
///
pub struct I2s<I, MODE> {
    instance: I,
    frame_format: FrameFormat,
    _mode: PhantomData<MODE>,
}

/// Marker for initialization mode
pub struct InitMode;
/// Marker for transmit mode
///
/// F is the data format
pub struct TransmitMode<F>(F);
/// Marker for receive mode
///
/// F is the data format
pub struct ReceiveMode<F>(F);

mod sealed {
    pub trait Sealed {}
}
use self::sealed::Sealed;

/// A mode in which the I2S is configured and may be enabled (transmit or receive)
pub trait ActiveMode: Sealed {}
impl<F> Sealed for TransmitMode<F> {}
impl<F> ActiveMode for TransmitMode<F> {}
impl<F> Sealed for ReceiveMode<F> {}
impl<F> ActiveMode for ReceiveMode<F> {}

impl<I, MODE> I2s<I, MODE>
where
    I: Instance,
{
    /// Returns a reference to the enclosed peripheral instance
    pub fn instance(&self) -> &I {
        &self.instance
    }
    /// Returns a mutable reference to the enclosed peripheral instance
    pub fn instance_mut(&mut self) -> &mut I {
        &mut self.instance
    }

    /// Returns a reference to the register block
    fn registers(&self) -> &RegisterBlock {
        unsafe { &*I::REGISTERS }
    }

    /// Enables the I2S peripheral
    fn common_enable(&self) {
        self.registers().i2scfgr.modify(|_, w| w.i2se().enabled());
    }

    /// Disables the I2S peripheral
    fn common_disable(&self) {
        self.registers().i2scfgr.modify(|_, w| w.i2se().disabled());
    }

    /// Resets the values of all control and configuration registers
    fn reset_registers(&self) {
        let registers = self.registers();
        registers.cr1.reset();
        registers.cr2.reset();
        registers.i2scfgr.reset();
        registers.i2spr.reset();
    }
}

impl<I> I2s<I, InitMode>
where
    I: Instance,
{
    /// Creates a wrapper around an instance, but does not do any configuration
    pub fn new(instance: I) -> Self {
        I2s {
            instance,
            // Default frame format (the real value will be filled in during configuration)
            frame_format: FrameFormat::PhilipsI2s,
            _mode: PhantomData,
        }
    }

    /// Configures the SPI peripheral in master transmit mode
    pub fn configure_master_transmit<F>(self, config: MasterConfig<F>) -> I2s<I, TransmitMode<F>>
    where
        F: DataFormat,
    {
        self.configure_clock_division(config.division, config.master_clock);
        self.configure_i2s(
            I2SCFG_A::MASTERTX,
            config.data_format,
            &config.frame_format,
            config.polarity,
        );
        I2s {
            instance: self.instance,
            frame_format: config.frame_format,
            _mode: PhantomData,
        }
    }

    /// Configures the SPI peripheral in master receive mode
    pub fn configure_master_receive<F>(self, config: MasterConfig<F>) -> I2s<I, ReceiveMode<F>>
    where
        F: DataFormat,
    {
        self.configure_clock_division(config.division, config.master_clock);
        self.configure_i2s(
            I2SCFG_A::MASTERRX,
            config.data_format,
            &config.frame_format,
            config.polarity,
        );
        I2s {
            instance: self.instance,
            frame_format: config.frame_format,
            _mode: PhantomData,
        }
    }

    /// Configures the SPI peripheral in slave transmit mode
    pub fn configure_slave_transmit<F>(self, config: SlaveConfig<F>) -> I2s<I, TransmitMode<F>>
    where
        F: DataFormat,
    {
        self.configure_i2s(
            I2SCFG_A::SLAVETX,
            config.data_format,
            &config.frame_format,
            config.polarity,
        );
        I2s {
            instance: self.instance,
            frame_format: config.frame_format,
            _mode: PhantomData,
        }
    }

    /// Configures the SPI peripheral in slave receive mode
    pub fn configure_slave_receive<F>(self, config: SlaveConfig<F>) -> I2s<I, ReceiveMode<F>>
    where
        F: DataFormat,
    {
        self.configure_i2s(
            I2SCFG_A::SLAVERX,
            config.data_format,
            &config.frame_format,
            config.polarity,
        );
        I2s {
            instance: self.instance,
            frame_format: config.frame_format,
            _mode: PhantomData,
        }
    }

    /// Sets the SPI peripheral to I2S mode and applies other settings to the SPI_CR2 register
    ///
    /// This does not modify any other registers, so it preserves interrupts and DMA setup.
    fn configure_i2s<F>(
        &self,
        mode: I2SCFG_A,
        _data_format: F,
        frame_format: &FrameFormat,
        polarity: Polarity,
    ) where
        F: DataFormat,
    {
        use self::pac::spi1::i2scfgr::{CKPOL_A, I2SSTD_A, PCMSYNC_A};
        let polarity = match polarity {
            Polarity::IdleLow => CKPOL_A::IDLELOW,
            Polarity::IdleHigh => CKPOL_A::IDLEHIGH,
        };
        let (i2sstd, pcmsync) = match frame_format {
            FrameFormat::PhilipsI2s => (I2SSTD_A::PHILIPS, PCMSYNC_A::SHORT),
            FrameFormat::MsbJustified => (I2SSTD_A::MSB, PCMSYNC_A::SHORT),
            FrameFormat::LsbJustified => (I2SSTD_A::LSB, PCMSYNC_A::SHORT),
            FrameFormat::Pcm(FrameSync::Short) => (I2SSTD_A::PCM, PCMSYNC_A::SHORT),
            FrameFormat::Pcm(FrameSync::Long) => (I2SSTD_A::PCM, PCMSYNC_A::LONG),
        };
        self.registers().i2scfgr.write(|w| {
            // Initially disabled (enable to actually start transferring data)
            w.i2se()
                .disabled()
                .i2smod()
                .i2smode()
                .i2scfg()
                .variant(mode)
                .pcmsync()
                .variant(pcmsync)
                .i2sstd()
                .variant(i2sstd)
                .ckpol()
                .variant(polarity)
                .datlen()
                .variant(F::DATLEN)
                .chlen()
                .variant(F::CHLEN)
        });
    }

    fn configure_clock_division(&self, division: u16, master_clock: MasterClock) {
        let master_clock_enable = matches!(master_clock, MasterClock::Enable);

        let spi = self.registers();
        let i2sdiv = division / 2;
        let odd = division % 2;
        assert!(i2sdiv >= 2 && i2sdiv <= 255);
        spi.i2spr.write(|w| unsafe {
            w.i2sdiv()
                .bits(i2sdiv as u8)
                .odd()
                .bit(odd != 0)
                .mckoe()
                .bit(master_clock_enable)
        });
    }
}

/// # Transmit mode
///
/// Both master and slave mode use the same functions to transmit. The only difference is where
/// the clock is generated.
///
/// ## Slave transmit
///
/// The I2S peripheral must be enabled and the first sample should be written to the transmit
/// register before the master starts sending clock and word select signals.
///
/// ## Master transmit
///
/// The first sample should be written to the transmit register just after the I2S peripheral is enabled.
/// Once the I2S peripheral is enabled, the first sample will be transmitted and the next sample
/// should be written to the transmit register.
///
impl<I, F> I2s<I, TransmitMode<F>>
where
    I: Instance,
    F: DataFormat,
{
    /// Returns the channel on which the next sample will be transmitted, or None if a previous
    /// sample is still in the process of being transmitted
    pub fn ready_to_transmit(&self) -> Option<Channel> {
        use self::pac::spi1::sr::CHSIDE_A;
        let registers = self.registers();
        let sr = registers.sr.read();
        if sr.txe().is_empty() {
            let channel = match sr.chside().variant() {
                CHSIDE_A::LEFT => Channel::Left,
                CHSIDE_A::RIGHT => Channel::Right,
            };
            Some(channel)
        } else {
            // Not ready, channel not valid
            None
        }
    }

    /// Writes a sample into the transmit buffer
    ///
    /// The I2S peripheral should normally be enabled before this function is called. However,
    /// if the data format contains 16 bits, this function can be called once before enabling the
    /// I2S to load the first sample.
    ///
    /// If the data format contains 24 or 32 bits, the sample will be split into two write
    /// operations. This function will block until the second write has completed.
    ///
    pub fn transmit(&mut self, sample: F::Sample) -> nb::Result<(), Infallible> {
        let registers = self.registers();
        let sr = registers.sr.read();
        if sr.txe().is_empty() {
            F::write_sample(&self.frame_format, &registers, sample);
            Ok(())
        } else {
            // Can't write yet
            Err(nb::Error::WouldBlock)
        }
    }

    /// Transmits multiple samples, blocking until all samples have been transmitted
    pub fn transmit_blocking(&mut self, samples: &[F::Sample]) {
        for sample in samples {
            nb::block!(self.transmit(*sample)).unwrap();
        }
    }

    /// Writes a 16-bit value to the data register
    ///
    /// Like `transmit`, this function returns `Err(nb::Error::WouldBlock)` if the data register
    /// contains a value that has not been transmitted yet.
    ///
    /// Unlike `transmit`, this function never blocks because it performs only one 16-bit write.
    /// If the data format contains 24 or 32 bits, the calling code is responsible for dividing
    /// each sample into two chunks and calling this function twice. Details about this can be found
    /// in the microcontroller reference manual.
    pub fn write_data_register(&mut self, value: u16) -> nb::Result<(), Infallible> {
        let registers = self.registers();
        let sr = registers.sr.read();
        if sr.txe().is_empty() {
            registers.dr.write(|w| w.dr().bits(value));
            Ok(())
        } else {
            // Can't write yet
            Err(nb::Error::WouldBlock)
        }
    }

    /// Checks for an error and clears the error flag
    pub fn take_error(&mut self) -> Result<(), TransmitError> {
        let spi = self.registers();
        // This read also clears the underrun flag
        let sr = spi.sr.read();
        if sr.udr().is_underrun() {
            Err(TransmitError::Underrun)
        } else {
            Ok(())
        }
    }

    /// Enables or disables DMA requests for transmission
    pub fn set_dma_enabled(&mut self, enabled: bool) {
        self.registers().cr2.modify(|_, w| w.txdmaen().bit(enabled));
    }

    /// Enables the I2S peripheral
    ///
    /// In master mode, this will activate the word select and clock outputs and start sending
    /// samples, with the left channel first. The first sample should be transmitted immediately
    /// after enabling the I2S.
    ///
    /// In slave mode, this will cause the I2S peripheral to start responding to word select
    /// and clock inputs from the master device. The first sample should be written to the data
    /// register before the word select input goes low.
    pub fn enable(&mut self) {
        self.common_enable();
    }

    /// Disables the I2S peripheral
    ///
    /// To avoid stopping a transfer in the middle of a frame, this function returns WouldBlock
    /// until the current transfer is finished.
    pub fn disable(&mut self) -> nb::Result<(), Infallible> {
        // "To switch off the I2S, by clearing I2SE, it is mandatory to wait for TXE = 1 and BSY = 0."
        let sr = self.registers().sr.read();
        if sr.txe().is_empty() && sr.bsy().is_not_busy() {
            self.common_disable();
            Ok(())
        } else {
            Err(nb::Error::WouldBlock)
        }
    }

    /// Returns the I2S to init mode, allowing it to be reconfigured
    ///
    /// This function resets all configuration options, including interrupts and DMA setup.
    ///
    /// If the I2S peripheral is enabled, this function will block until it has finished the
    /// current transmission.
    pub fn deconfigure(mut self) -> I2s<I, InitMode> {
        nb::block!(self.disable()).unwrap();
        self.reset_registers();
        I2s {
            instance: self.instance,
            // Default frame format (the real value will be filled in during configuration)
            frame_format: FrameFormat::PhilipsI2s,
            _mode: PhantomData,
        }
    }
}

/// # Receive mode
///
/// Both master and slave mode use the same functions to receive. The only difference is where
/// the clock is generated.
///
impl<I, F> I2s<I, ReceiveMode<F>>
where
    I: Instance,
    F: DataFormat,
{
    /// Enables the I2S peripheral
    ///
    /// In master mode, this will activate the word select and clock outputs and start receiving
    /// samples, with the left channel first. The first sample will be available shortly
    /// after enabling the I2S.
    ///
    /// In slave mode, this will cause the I2S peripheral to start responding to word select
    /// and clock inputs from the master device.
    pub fn enable(&mut self) {
        self.common_enable();
    }

    /// If a sample has been read in and is ready to receive, this function returns the channel
    /// it was received on.
    pub fn sample_ready(&self) -> Option<Channel> {
        use crate::pac::spi1::sr::CHSIDE_A;

        let spi = self.registers();
        let sr = spi.sr.read();
        if sr.rxne().is_not_empty() {
            let channel = match sr.chside().variant() {
                CHSIDE_A::LEFT => Channel::Left,
                CHSIDE_A::RIGHT => Channel::Right,
            };
            Some(channel)
        } else {
            None
        }
    }

    /// Receives a sample from the data register, returning the sample and its associated channel
    ///
    /// If the data format contains 24 or 32 bits, the sample will be split into two read
    /// operations. This function will block until the second read has completed.
    pub fn receive(&mut self) -> nb::Result<(F::Sample, Channel), Infallible> {
        match self.sample_ready() {
            Some(channel) => {
                let sample = F::read_sample(&self.frame_format, self.registers());
                Ok((sample, channel))
            }
            None => Err(nb::Error::WouldBlock),
        }
    }

    /// Receives multiple samples, blocking until all samples have been received
    ///
    /// Samples from the left and right channels will be interleaved.
    pub fn receive_blocking(&mut self, samples: &mut [F::Sample]) {
        for sample_in_buffer in samples {
            let (sample, _channel) = nb::block!(self.receive()).unwrap();
            *sample_in_buffer = sample;
        }
    }

    /// Reads a 16-bit value from the data register, returning the value and its associated channel
    ///
    /// Like `receive`, this function returns `Err(nb::Error::WouldBlock)` if the data register
    /// does not contain a value.
    ///
    /// Unlike `receive`, this function never blocks because it performs only one 16-bit read.
    /// If the data format contains 24 or 32 bits, the calling code is responsible for calling this
    /// function twice and combining the two returned chunks into a sample. Details about this can
    /// be found in the microcontroller reference manual.
    pub fn read_data_register(&mut self) -> nb::Result<(u16, Channel), Infallible> {
        match self.sample_ready() {
            Some(channel) => {
                let sample = self.registers().dr.read().dr().bits();
                Ok((sample, channel))
            }
            None => Err(nb::Error::WouldBlock),
        }
    }

    /// Checks if an error has occurred, and clears the overrun error flag
    pub fn take_error(&mut self) -> Result<(), ReceiveError> {
        let spi = self.registers();
        let sr = spi.sr.read();
        let frame_error = sr.fre().is_error();
        let overrun = sr.ovr().is_overrun();
        if overrun {
            // Clear flag by reading DR and then SR
            let dr = spi.dr.read();
            let _ = spi.sr.read();
            if frame_error {
                Err(ReceiveError::FrameAndOverrun(dr.dr().bits))
            } else {
                Err(ReceiveError::Overrun(dr.dr().bits))
            }
        } else if frame_error {
            Err(ReceiveError::Frame)
        } else {
            Ok(())
        }
    }

    /// Enables or disables DMA requests for reception
    pub fn set_dma_enabled(&mut self, enabled: bool) {
        self.registers().cr2.modify(|_, w| w.rxdmaen().bit(enabled));
    }

    /// Disables the I2S
    ///
    /// In master mode, this stops the clock, word select, and (if enabled) master clock outputs.
    ///
    /// Caution: Before disabling the I2S, a specific sequence of operations should be performed
    /// so that the I2S peripheral does not stop in the middle of a frame. Refer to the target
    /// microcontroller reference manual for more information.
    pub fn disable(&mut self) {
        self.common_disable();
    }

    /// Returns the I2S to init mode, allowing it to be reconfigured
    ///
    /// This function resets all configuration options, including interrupts and DMA setup.
    ///
    /// If the I2S peripheral is enabled, this function will disable it.
    pub fn deconfigure(mut self) -> I2s<I, InitMode> {
        self.disable();
        self.reset_registers();
        I2s {
            instance: self.instance,
            // Default frame format (the real value will be filled in during configuration)
            frame_format: FrameFormat::PhilipsI2s,
            _mode: PhantomData,
        }
    }
}

/// # Common functions
///
/// These interrupt functions can be used for transmission and reception.
impl<I, M> I2s<I, M>
where
    I: Instance,
    M: ActiveMode,
{
    /// Enables the interrupt signal output for an event
    pub fn listen(&mut self, event: Event) {
        self.registers().cr2.modify(|_, w| match event {
            Event::TransmitEmtpy => w.txeie().not_masked(),
            Event::ReceiveNotEmpty => w.rxneie().not_masked(),
            Event::Error => w.errie().not_masked(),
        })
    }
    /// Disables the interrupt signal output for an event
    pub fn unlisten(&mut self, event: Event) {
        self.registers().cr2.modify(|_, w| match event {
            Event::TransmitEmtpy => w.txeie().masked(),
            Event::ReceiveNotEmpty => w.rxneie().masked(),
            Event::Error => w.errie().masked(),
        })
    }
}

//Notes:
//  - I is a tuple (SPI, I2SEXT)
//  - MODES is a tuple (SPIMODE, I2SMODE)
pub struct DualI2s<I, MODES> {
    instance: I,
    frame_formats: (FrameFormat, FrameFormat),
    _modes: PhantomData<MODES>,
}

impl<I, MODES> DualI2s<I, MODES>
where
    I: DualInstance,
{
    /// Returns references to the enclosed peripheral instances
    pub fn instance(&self) -> &I {
        &self.instance
    }
    /// Returns mutable references to the enclosed peripheral instances
    pub fn instance_mut(&mut self) -> &mut I {
        &mut self.instance
    }

    /// Returns references to the main I2S register blocks
    fn main_registers(&self) -> &RegisterBlock {
        unsafe { &*I::REGISTERS.0 }
    }

    /// Returns references to the extended I2S register blocks
    fn ext_registers(&self) -> &RegisterBlock {
        unsafe { &*I::REGISTERS.1 }
    }

    /// Enables the main I2S peripheral
    fn main_common_enable(&self) {
        self.main_registers()
            .i2scfgr
            .modify(|_, w| w.i2se().enabled());
    }

    /// Enables the extended I2S peripheral
    fn ext_common_enable(&self) {
        self.ext_registers()
            .i2scfgr
            .modify(|_, w| w.i2se().enabled());
    }

    /// Disables the main I2S peripheral
    fn main_common_disable(&self) {
        self.main_registers()
            .i2scfgr
            .modify(|_, w| w.i2se().disabled());
    }

    /// Disables the extended I2S peripheral
    fn ext_common_disable(&self) {
        self.ext_registers()
            .i2scfgr
            .modify(|_, w| w.i2se().disabled());
    }

    /// Resets the values of all control and configuration registers
    fn main_reset_registers(&self) {
        let registers = self.main_registers();
        registers.cr1.reset();
        registers.cr2.reset();
        registers.i2scfgr.reset();
        registers.i2spr.reset();
    }

    /// Resets the values of all control and configuration registers
    fn ext_reset_registers(&self) {
        let registers = self.ext_registers();
        registers.cr1.reset();
        registers.cr2.reset();
        registers.i2scfgr.reset();
        registers.i2spr.reset();
    }
}

impl<I> DualI2s<I, (InitMode, InitMode)>
where
    I: DualInstance,
{
    /// Creates a wrapper around SPI and I2SEXT peripherals, but does not do any configuration
    pub fn new(instance: I) -> Self {
        DualI2s {
            instance,
            // Default frame format (the real value will be filled in during configuration)
            frame_formats: (FrameFormat::PhilipsI2s, FrameFormat::PhilipsI2s),
            _modes: PhantomData,
        }
    }
}

// Main part configuration
impl<I, ANYMODE> DualI2s<I, (InitMode, ANYMODE)>
where
    I: DualInstance,
{
    /// Configures the main peripheral in master transmit mode
    pub fn main_configure_master_transmit<F>(
        self,
        config: MasterConfig<F>,
    ) -> DualI2s<I, (TransmitMode<F>, ANYMODE)>
    where
        F: DataFormat,
    {
        self.main_configure_clock_division(config.division, config.master_clock);
        self.main_configure_i2s(
            I2SCFG_A::MASTERTX,
            config.data_format,
            &config.frame_format,
            config.polarity,
        );
        DualI2s {
            instance: self.instance,
            frame_formats: (config.frame_format, self.frame_formats.1),
            _modes: PhantomData,
        }
    }

    /// Configures the main peripheral in master receive mode
    pub fn main_configure_master_receive<F>(
        self,
        config: MasterConfig<F>,
    ) -> DualI2s<I, (ReceiveMode<F>, ANYMODE)>
    where
        F: DataFormat,
    {
        self.main_configure_clock_division(config.division, config.master_clock);
        self.main_configure_i2s(
            I2SCFG_A::MASTERRX,
            config.data_format,
            &config.frame_format,
            config.polarity,
        );
        DualI2s {
            instance: self.instance,
            frame_formats: (config.frame_format, self.frame_formats.1),
            _modes: PhantomData,
        }
    }

    /// Configures the SPI peripheral in slave transmit mode
    pub fn main_configure_slave_transmit<F>(
        self,
        config: SlaveConfig<F>,
    ) -> DualI2s<I, (TransmitMode<F>, ANYMODE)>
    where
        F: DataFormat,
    {
        self.main_configure_i2s(
            I2SCFG_A::SLAVETX,
            config.data_format,
            &config.frame_format,
            config.polarity,
        );
        DualI2s {
            instance: self.instance,
            frame_formats: (config.frame_format, self.frame_formats.1),
            _modes: PhantomData,
        }
    }

    /// Configures the SPI peripheral in slave receive mode
    pub fn main_configure_slave_receive<F>(
        self,
        config: SlaveConfig<F>,
    ) -> DualI2s<I, (ReceiveMode<F>, ANYMODE)>
    where
        F: DataFormat,
    {
        self.main_configure_i2s(
            I2SCFG_A::SLAVERX,
            config.data_format,
            &config.frame_format,
            config.polarity,
        );
        DualI2s {
            instance: self.instance,
            frame_formats: (config.frame_format, self.frame_formats.1),
            _modes: PhantomData,
        }
    }

    /// Sets the SPI peripheral to I2S mode and applies other settings to the SPI_CR2 register
    ///
    /// This does not modify any other registers, so it preserves interrupts and DMA setup.
    fn main_configure_i2s<F>(
        &self,
        mode: I2SCFG_A,
        _data_format: F,
        frame_format: &FrameFormat,
        polarity: Polarity,
    ) where
        F: DataFormat,
    {
        use self::pac::spi1::i2scfgr::{CKPOL_A, I2SSTD_A, PCMSYNC_A};
        let polarity = match polarity {
            Polarity::IdleLow => CKPOL_A::IDLELOW,
            Polarity::IdleHigh => CKPOL_A::IDLEHIGH,
        };
        let (i2sstd, pcmsync) = match frame_format {
            FrameFormat::PhilipsI2s => (I2SSTD_A::PHILIPS, PCMSYNC_A::SHORT),
            FrameFormat::MsbJustified => (I2SSTD_A::MSB, PCMSYNC_A::SHORT),
            FrameFormat::LsbJustified => (I2SSTD_A::LSB, PCMSYNC_A::SHORT),
            FrameFormat::Pcm(FrameSync::Short) => (I2SSTD_A::PCM, PCMSYNC_A::SHORT),
            FrameFormat::Pcm(FrameSync::Long) => (I2SSTD_A::PCM, PCMSYNC_A::LONG),
        };
        self.main_registers().i2scfgr.write(|w| {
            // Initially disabled (enable to actually start transferring data)
            w.i2se().disabled();
            w.i2smod().i2smode();
            w.i2scfg().variant(mode);
            w.pcmsync().variant(pcmsync);
            w.i2sstd().variant(i2sstd);
            w.ckpol().variant(polarity);
            w.datlen().variant(F::DATLEN);
            w.chlen().variant(F::CHLEN)
        });
    }

    fn main_configure_clock_division(&self, division: u16, master_clock: MasterClock) {
        let master_clock_enable = matches!(master_clock, MasterClock::Enable);

        let spi = self.main_registers();
        let i2sdiv = division / 2;
        let odd = division % 2;
        assert!(i2sdiv >= 2 && i2sdiv <= 255);
        spi.i2spr.write(|w| unsafe {
            w.i2sdiv().bits(i2sdiv as u8);
            w.odd().bit(odd != 0);
            w.mckoe().bit(master_clock_enable)
        });
    }
}

// extended part configuration
impl<I, ANYMODE> DualI2s<I, (ANYMODE, InitMode)>
where
    I: DualInstance,
{
    /// Configures the SPI peripheral in slave transmit mode
    pub fn ext_configure_slave_transmit<F>(
        self,
        config: SlaveConfig<F>,
    ) -> DualI2s<I, (ANYMODE, TransmitMode<F>)>
    where
        F: DataFormat,
    {
        self.ext_configure_i2s(
            I2SCFG_A::SLAVETX,
            config.data_format,
            &config.frame_format,
            config.polarity,
        );
        DualI2s {
            instance: self.instance,
            frame_formats: (self.frame_formats.0, config.frame_format),
            _modes: PhantomData,
        }
    }

    /// Configures the SPI peripheral in slave receive mode
    pub fn ext_configure_slave_receive<F>(
        self,
        config: SlaveConfig<F>,
    ) -> DualI2s<I, (ANYMODE, TransmitMode<F>)>
    where
        F: DataFormat,
    {
        self.ext_configure_i2s(
            I2SCFG_A::SLAVERX,
            config.data_format,
            &config.frame_format,
            config.polarity,
        );
        DualI2s {
            instance: self.instance,
            frame_formats: (self.frame_formats.0, config.frame_format),
            _modes: PhantomData,
        }
    }

    /// Sets the SPI peripheral to I2S mode and applies other settings to the SPI_CR2 register
    ///
    /// This does not modify any other registers, so it preserves interrupts and DMA setup.
    fn ext_configure_i2s<F>(
        &self,
        mode: I2SCFG_A,
        _data_format: F,
        frame_format: &FrameFormat,
        polarity: Polarity,
    ) where
        F: DataFormat,
    {
        use self::pac::spi1::i2scfgr::{CKPOL_A, I2SSTD_A, PCMSYNC_A};
        let polarity = match polarity {
            Polarity::IdleLow => CKPOL_A::IDLELOW,
            Polarity::IdleHigh => CKPOL_A::IDLEHIGH,
        };
        let (i2sstd, pcmsync) = match frame_format {
            FrameFormat::PhilipsI2s => (I2SSTD_A::PHILIPS, PCMSYNC_A::SHORT),
            FrameFormat::MsbJustified => (I2SSTD_A::MSB, PCMSYNC_A::SHORT),
            FrameFormat::LsbJustified => (I2SSTD_A::LSB, PCMSYNC_A::SHORT),
            FrameFormat::Pcm(FrameSync::Short) => (I2SSTD_A::PCM, PCMSYNC_A::SHORT),
            FrameFormat::Pcm(FrameSync::Long) => (I2SSTD_A::PCM, PCMSYNC_A::LONG),
        };
        self.ext_registers().i2scfgr.write(|w| {
            // Initially disabled (enable to actually start transferring data)
            w.i2se().disabled();
            w.i2smod().i2smode();
            w.i2scfg().variant(mode);
            w.pcmsync().variant(pcmsync);
            w.i2sstd().variant(i2sstd);
            w.ckpol().variant(polarity);
            w.datlen().variant(F::DATLEN);
            w.chlen().variant(F::CHLEN)
        });
    }
}

// main half transmit
impl<I, ANYMODE, F> DualI2s<I, (TransmitMode<F>, ANYMODE)>
where
    I: DualInstance,
    F: DataFormat,
{
    /// Returns the channel on which the next sample will be transmitted, or None if a previous
    /// sample is still in the process of being transmitted
    pub fn main_ready_to_transmit(&self) -> Option<Channel> {
        use self::pac::spi1::sr::CHSIDE_A;
        let registers = self.main_registers();
        let sr = registers.sr.read();
        if sr.txe().is_empty() {
            let channel = match sr.chside().variant() {
                CHSIDE_A::LEFT => Channel::Left,
                CHSIDE_A::RIGHT => Channel::Right,
            };
            Some(channel)
        } else {
            // Not ready, channel not valid
            None
        }
    }

    /// Writes a sample into the transmit buffer
    ///
    /// The I2S peripheral should normally be enabled before this function is called. However,
    /// if the data format contains 16 bits, this function can be called once before enabling the
    /// I2S to load the first sample.
    ///
    /// If the data format contains 24 or 32 bits, the sample will be split into two write
    /// operations. This function will block until the second write has completed.
    ///
    pub fn main_transmit(&mut self, sample: F::Sample) -> nb::Result<(), Infallible> {
        let registers = self.main_registers();
        let sr = registers.sr.read();
        if sr.txe().is_empty() {
            F::write_sample(&self.frame_formats.0, &registers, sample);
            Ok(())
        } else {
            // Can't write yet
            Err(nb::Error::WouldBlock)
        }
    }

    /// Transmits multiple samples, blocking until all samples have been transmitted
    pub fn main_transmit_blocking(&mut self, samples: &[F::Sample]) {
        for sample in samples {
            nb::block!(self.main_transmit(*sample)).unwrap();
        }
    }

    /// Writes a 16-bit value to the data register
    ///
    /// Like `transmit`, this function returns `Err(nb::Error::WouldBlock)` if the data register
    /// contains a value that has not been transmitted yet.
    ///
    /// Unlike `transmit`, this function never blocks because it performs only one 16-bit write.
    /// If the data format contains 24 or 32 bits, the calling code is responsible for dividing
    /// each sample into two chunks and calling this function twice. Details about this can be found
    /// in the microcontroller reference manual.
    pub fn main_write_data_register(&mut self, value: u16) -> nb::Result<(), Infallible> {
        let registers = self.main_registers();
        let sr = registers.sr.read();
        if sr.txe().is_empty() {
            registers.dr.write(|w| w.dr().bits(value));
            Ok(())
        } else {
            // Can't write yet
            Err(nb::Error::WouldBlock)
        }
    }

    /// Checks for an error and clears the error flag
    pub fn main_take_error(&mut self) -> Result<(), TransmitError> {
        let spi = self.main_registers();
        // This read also clears the underrun flag
        let sr = spi.sr.read();
        if sr.udr().is_underrun() {
            Err(TransmitError::Underrun)
        } else {
            Ok(())
        }
    }

    /// Enables or disables DMA requests for transmission
    pub fn main_set_dma_enabled(&mut self, enabled: bool) {
        self.main_registers()
            .cr2
            .modify(|_, w| w.txdmaen().bit(enabled));
    }

    /// Enables the I2S peripheral
    ///
    /// In master mode, this will activate the word select and clock outputs and start sending
    /// samples, with the left channel first. The first sample should be transmitted immediately
    /// after enabling the I2S.
    ///
    /// In slave mode, this will cause the I2S peripheral to start responding to word select
    /// and clock inputs from the master device. The first sample should be written to the data
    /// register before the word select input goes low.
    pub fn main_enable(&mut self) {
        self.main_common_enable();
    }

    /// Disables the I2S peripheral
    ///
    /// To avoid stopping a transfer in the middle of a frame, this function returns WouldBlock
    /// until the current transfer is finished.
    pub fn main_disable(&mut self) -> nb::Result<(), Infallible> {
        // "To switch off the I2S, by clearing I2SE, it is mandatory to wait for TXE = 1 and BSY = 0."
        let sr = self.main_registers().sr.read();
        if sr.txe().is_empty() && sr.bsy().is_not_busy() {
            self.main_common_disable();
            Ok(())
        } else {
            Err(nb::Error::WouldBlock)
        }
    }

    /// Returns the I2S to init mode, allowing it to be reconfigured
    ///
    /// This function resets all configuration options, including interrupts and DMA setup.
    ///
    /// If the I2S peripheral is enabled, this function will block until it has finished the
    /// current transmission.
    pub fn main_deconfigure(mut self) -> DualI2s<I, InitMode> {
        nb::block!(self.main_disable()).unwrap();
        self.main_reset_registers();
        DualI2s {
            instance: self.instance,
            // Default frame format (the real value will be filled in during configuration)
            frame_formats: (FrameFormat::PhilipsI2s, self.frame_formats.1),
            _modes: PhantomData,
        }
    }
}

// ext half transmit
impl<I, ANYMODE, F> DualI2s<I, (ANYMODE, TransmitMode<F>)>
where
    I: DualInstance,
    F: DataFormat,
{
    /// Returns the channel on which the next sample will be transmitted, or None if a previous
    /// sample is still in the process of being transmitted
    pub fn ext_ready_to_transmit(&self) -> Option<Channel> {
        use self::pac::spi1::sr::CHSIDE_A;
        let registers = self.ext_registers();
        let sr = registers.sr.read();
        if sr.txe().is_empty() {
            let channel = match sr.chside().variant() {
                CHSIDE_A::LEFT => Channel::Left,
                CHSIDE_A::RIGHT => Channel::Right,
            };
            Some(channel)
        } else {
            // Not ready, channel not valid
            None
        }
    }

    /// Writes a sample into the transmit buffer
    ///
    /// The I2S peripheral should normally be enabled before this function is called. However,
    /// if the data format contains 16 bits, this function can be called once before enabling the
    /// I2S to load the first sample.
    ///
    /// If the data format contains 24 or 32 bits, the sample will be split into two write
    /// operations. This function will block until the second write has completed.
    ///
    pub fn ext_transmit(&mut self, sample: F::Sample) -> nb::Result<(), Infallible> {
        let registers = self.ext_registers();
        let sr = registers.sr.read();
        if sr.txe().is_empty() {
            F::write_sample(&self.frame_formats.0, &registers, sample);
            Ok(())
        } else {
            // Can't write yet
            Err(nb::Error::WouldBlock)
        }
    }

    /// Transmits multiple samples, blocking until all samples have been transmitted
    pub fn ext_transmit_blocking(&mut self, samples: &[F::Sample]) {
        for sample in samples {
            nb::block!(self.ext_transmit(*sample)).unwrap();
        }
    }

    /// Writes a 16-bit value to the data register
    ///
    /// Like `transmit`, this function returns `Err(nb::Error::WouldBlock)` if the data register
    /// contains a value that has not been transmitted yet.
    ///
    /// Unlike `transmit`, this function never blocks because it performs only one 16-bit write.
    /// If the data format contains 24 or 32 bits, the calling code is responsible for dividing
    /// each sample into two chunks and calling this function twice. Details about this can be found
    /// in the microcontroller reference manual.
    pub fn ext_write_data_register(&mut self, value: u16) -> nb::Result<(), Infallible> {
        let registers = self.ext_registers();
        let sr = registers.sr.read();
        if sr.txe().is_empty() {
            registers.dr.write(|w| w.dr().bits(value));
            Ok(())
        } else {
            // Can't write yet
            Err(nb::Error::WouldBlock)
        }
    }

    /// Checks for an error and clears the error flag
    pub fn ext_take_error(&mut self) -> Result<(), TransmitError> {
        let spi = self.ext_registers();
        // This read also clears the underrun flag
        let sr = spi.sr.read();
        if sr.udr().is_underrun() {
            Err(TransmitError::Underrun)
        } else {
            Ok(())
        }
    }

    /// Enables or disables DMA requests for transmission
    pub fn ext_set_dma_enabled(&mut self, enabled: bool) {
        self.ext_registers()
            .cr2
            .modify(|_, w| w.txdmaen().bit(enabled));
    }

    /// Enables the I2S peripheral
    ///
    /// In master mode, this will activate the word select and clock outputs and start sending
    /// samples, with the left channel first. The first sample should be transmitted immediately
    /// after enabling the I2S.
    ///
    /// In slave mode, this will cause the I2S peripheral to start responding to word select
    /// and clock inputs from the master device. The first sample should be written to the data
    /// register before the word select input goes low.
    pub fn ext_enable(&mut self) {
        self.ext_common_enable();
    }

    /// Disables the I2S peripheral
    ///
    /// To avoid stopping a transfer in the middle of a frame, this function returns WouldBlock
    /// until the current transfer is finished.
    pub fn ext_disable(&mut self) -> nb::Result<(), Infallible> {
        // "To switch off the I2S, by clearing I2SE, it is mandatory to wait for TXE = 1 and BSY = 0."
        let sr = self.ext_registers().sr.read();
        if sr.txe().is_empty() && sr.bsy().is_not_busy() {
            self.ext_common_disable();
            Ok(())
        } else {
            Err(nb::Error::WouldBlock)
        }
    }

    /// Returns the I2S to init mode, allowing it to be reconfigured
    ///
    /// This function resets all configuration options, including interrupts and DMA setup.
    ///
    /// If the I2S peripheral is enabled, this function will block until it has finished the
    /// current transmission.
    pub fn ext_deconfigure(mut self) -> DualI2s<I, InitMode> {
        nb::block!(self.ext_disable()).unwrap();
        self.ext_reset_registers();
        DualI2s {
            instance: self.instance,
            // Default frame format (the real value will be filled in during configuration)
            frame_formats: (self.frame_formats.0, FrameFormat::PhilipsI2s),
            _modes: PhantomData,
        }
    }
}

// Main half receive
impl<I, ANYMODE, F> DualI2s<I, (ReceiveMode<F>, ANYMODE)>
where
    I: DualInstance,
    F: DataFormat,
{
    /// Enables the I2S peripheral
    ///
    /// In master mode, this will activate the word select and clock outputs and start receiving
    /// samples, with the left channel first. The first sample will be available shortly
    /// after enabling the I2S.
    ///
    /// In slave mode, this will cause the I2S peripheral to start responding to word select
    /// and clock inputs from the master device.
    pub fn main_enable(&mut self) {
        self.main_common_enable();
    }

    /// If a sample has been read in and is ready to receive, this function returns the channel
    /// it was received on.
    pub fn main_sample_ready(&self) -> Option<Channel> {
        use crate::pac::spi1::sr::CHSIDE_A;

        let spi = self.main_registers();
        let sr = spi.sr.read();
        if sr.rxne().is_not_empty() {
            let channel = match sr.chside().variant() {
                CHSIDE_A::LEFT => Channel::Left,
                CHSIDE_A::RIGHT => Channel::Right,
            };
            Some(channel)
        } else {
            None
        }
    }

    /// Receives a sample from the data register, returning the sample and its associated channel
    ///
    /// If the data format contains 24 or 32 bits, the sample will be split into two read
    /// operations. This function will block until the second read has completed.
    pub fn main_receive(&mut self) -> nb::Result<(F::Sample, Channel), Infallible> {
        match self.main_sample_ready() {
            Some(channel) => {
                let sample = F::read_sample(&self.frame_formats.0, self.main_registers());
                Ok((sample, channel))
            }
            None => Err(nb::Error::WouldBlock),
        }
    }

    /// Receives multiple samples, blocking until all samples have been received
    ///
    /// Samples from the left and right channels will be interleaved.
    pub fn main_receive_blocking(&mut self, samples: &mut [F::Sample]) {
        for sample_in_buffer in samples {
            let (sample, _channel) = nb::block!(self.main_receive()).unwrap();
            *sample_in_buffer = sample;
        }
    }

    /// Reads a 16-bit value from the data register, returning the value and its associated channel
    ///
    /// Like `receive`, this function returns `Err(nb::Error::WouldBlock)` if the data register
    /// does not contain a value.
    ///
    /// Unlike `receive`, this function never blocks because it performs only one 16-bit read.
    /// If the data format contains 24 or 32 bits, the calling code is responsible for calling this
    /// function twice and combining the two returned chunks into a sample. Details about this can
    /// be found in the microcontroller reference manual.
    pub fn main_read_data_register(&mut self) -> nb::Result<(u16, Channel), Infallible> {
        match self.main_sample_ready() {
            Some(channel) => {
                let sample = self.main_registers().dr.read().dr().bits();
                Ok((sample, channel))
            }
            None => Err(nb::Error::WouldBlock),
        }
    }

    /// Checks if an error has occurred, and clears the overrun error flag
    pub fn main_take_error(&mut self) -> Result<(), ReceiveError> {
        let spi = self.main_registers();
        let sr = spi.sr.read();
        let frame_error = sr.fre().is_error();
        let overrun = sr.ovr().is_overrun();
        if overrun {
            // Clear flag by reading DR and then SR
            let dr = spi.dr.read();
            let _ = spi.sr.read();
            if frame_error {
                Err(ReceiveError::FrameAndOverrun(dr.dr().bits))
            } else {
                Err(ReceiveError::Overrun(dr.dr().bits))
            }
        } else if frame_error {
            Err(ReceiveError::Frame)
        } else {
            Ok(())
        }
    }

    /// Enables or disables DMA requests for reception
    pub fn main_set_dma_enabled(&mut self, enabled: bool) {
        self.main_registers()
            .cr2
            .modify(|_, w| w.rxdmaen().bit(enabled));
    }

    /// Disables the I2S
    ///
    /// In master mode, this stops the clock, word select, and (if enabled) master clock outputs.
    ///
    /// Caution: Before disabling the I2S, a specific sequence of operations should be performed
    /// so that the I2S peripheral does not stop in the middle of a frame. Refer to the target
    /// microcontroller reference manual for more information.
    pub fn main_disable(&mut self) {
        self.main_common_disable();
    }

    /// Returns the I2S to init mode, allowing it to be reconfigured
    ///
    /// This function resets all configuration options, including interrupts and DMA setup.
    ///
    /// If the I2S peripheral is enabled, this function will disable it.
    pub fn main_deconfigure(mut self) -> DualI2s<I, (InitMode, ANYMODE)> {
        self.main_disable();
        self.main_reset_registers();
        DualI2s {
            instance: self.instance,
            // Default frame format (the real value will be filled in during configuration)
            frame_formats: (FrameFormat::PhilipsI2s, self.frame_formats.1),
            _modes: PhantomData,
        }
    }
}

// ext half receive
impl<I, ANYMODE, F> DualI2s<I, (ANYMODE, ReceiveMode<F>)>
where
    I: DualInstance,
    F: DataFormat,
{
    /// Enables the I2S peripheral
    ///
    /// In master mode, this will activate the word select and clock outputs and start receiving
    /// samples, with the left channel first. The first sample will be available shortly
    /// after enabling the I2S.
    ///
    /// In slave mode, this will cause the I2S peripheral to start responding to word select
    /// and clock inputs from the master device.
    pub fn ext_enable(&mut self) {
        self.ext_common_enable();
    }

    /// If a sample has been read in and is ready to receive, this function returns the channel
    /// it was received on.
    pub fn ext_sample_ready(&self) -> Option<Channel> {
        use crate::pac::spi1::sr::CHSIDE_A;

        let spi = self.ext_registers();
        let sr = spi.sr.read();
        if sr.rxne().is_not_empty() {
            let channel = match sr.chside().variant() {
                CHSIDE_A::LEFT => Channel::Left,
                CHSIDE_A::RIGHT => Channel::Right,
            };
            Some(channel)
        } else {
            None
        }
    }

    /// Receives a sample from the data register, returning the sample and its associated channel
    ///
    /// If the data format contains 24 or 32 bits, the sample will be split into two read
    /// operations. This function will block until the second read has completed.
    pub fn ext_receive(&mut self) -> nb::Result<(F::Sample, Channel), Infallible> {
        match self.ext_sample_ready() {
            Some(channel) => {
                let sample = F::read_sample(&self.frame_formats.0, self.ext_registers());
                Ok((sample, channel))
            }
            None => Err(nb::Error::WouldBlock),
        }
    }

    /// Receives multiple samples, blocking until all samples have been received
    ///
    /// Samples from the left and right channels will be interleaved.
    pub fn ext_receive_blocking(&mut self, samples: &mut [F::Sample]) {
        for sample_in_buffer in samples {
            let (sample, _channel) = nb::block!(self.ext_receive()).unwrap();
            *sample_in_buffer = sample;
        }
    }

    /// Reads a 16-bit value from the data register, returning the value and its associated channel
    ///
    /// Like `receive`, this function returns `Err(nb::Error::WouldBlock)` if the data register
    /// does not contain a value.
    ///
    /// Unlike `receive`, this function never blocks because it performs only one 16-bit read.
    /// If the data format contains 24 or 32 bits, the calling code is responsible for calling this
    /// function twice and combining the two returned chunks into a sample. Details about this can
    /// be found in the microcontroller reference manual.
    pub fn ext_read_data_register(&mut self) -> nb::Result<(u16, Channel), Infallible> {
        match self.ext_sample_ready() {
            Some(channel) => {
                let sample = self.ext_registers().dr.read().dr().bits();
                Ok((sample, channel))
            }
            None => Err(nb::Error::WouldBlock),
        }
    }

    /// Checks if an error has occurred, and clears the overrun error flag
    pub fn ext_take_error(&mut self) -> Result<(), ReceiveError> {
        let spi = self.ext_registers();
        let sr = spi.sr.read();
        let frame_error = sr.fre().is_error();
        let overrun = sr.ovr().is_overrun();
        if overrun {
            // Clear flag by reading DR and then SR
            let dr = spi.dr.read();
            let _ = spi.sr.read();
            if frame_error {
                Err(ReceiveError::FrameAndOverrun(dr.dr().bits))
            } else {
                Err(ReceiveError::Overrun(dr.dr().bits))
            }
        } else if frame_error {
            Err(ReceiveError::Frame)
        } else {
            Ok(())
        }
    }

    /// Enables or disables DMA requests for reception
    pub fn ext_set_dma_enabled(&mut self, enabled: bool) {
        self.ext_registers()
            .cr2
            .modify(|_, w| w.rxdmaen().bit(enabled));
    }

    /// Disables the I2S
    ///
    /// In master mode, this stops the clock, word select, and (if enabled) master clock outputs.
    ///
    /// Caution: Before disabling the I2S, a specific sequence of operations should be performed
    /// so that the I2S peripheral does not stop in the middle of a frame. Refer to the target
    /// microcontroller reference manual for more information.
    pub fn ext_disable(&mut self) {
        self.ext_common_disable();
    }

    /// Returns the I2S to init mode, allowing it to be reconfigured
    ///
    /// This function resets all configuration options, including interrupts and DMA setup.
    ///
    /// If the I2S peripheral is enabled, this function will disable it.
    pub fn ext_deconfigure(mut self) -> DualI2s<I, (ANYMODE, InitMode)> {
        self.ext_disable();
        self.ext_reset_registers();
        DualI2s {
            instance: self.instance,
            // Default frame format (the real value will be filled in during configuration)
            frame_formats: (self.frame_formats.0, FrameFormat::PhilipsI2s),
            _modes: PhantomData,
        }
    }
}

/// Errors that can occur when transmitting
#[derive(Debug)]
pub enum TransmitError {
    /// The I2S peripheral needed to transmit a sample but no sample had been written
    /// to the data register
    ///
    /// This indicates that at least one incorrect sample was transmitted
    Underrun,
}

/// Errors that can occur when receiving
#[derive(Debug)]
pub enum ReceiveError {
    /// The I2S peripheral received a sample before software read the previous sample
    ///
    /// This indicates that at least one sample was lost.
    ///
    /// The enclosed value is the 16-bit value in the data register when overrun first happened.
    /// Depending on the data format, this may be a full sample or just part of a sample.
    /// The following samples have been discarded.
    Overrun(u16),
    /// The word select signal changed at an unexpected time (for slave mode only)
    ///
    /// If this error occurs, the I2S peripheral should be disabled and then re-enabled when
    /// the word select signal is high.
    Frame,
    /// Both frame and overrun errors were detected
    FrameAndOverrun(u16),
}
