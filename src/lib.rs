//! This library supports I2S communication for SPI version 1.2 (on STM32F1, STM32F2, STM32F4,
//! STM32L0, and STM32L1 microcontrollers).
//!
//! This library is normally used with a HAL library that provides a type that implements
//! [I2sPeripheral](crate::I2sPeripheral). An [I2sDriver](crate::I2sDriver) object can be created around the I2sPeripheral
//! object and used for I2S.

#![no_std]

extern crate nb;
extern crate vcell;

mod config;
pub mod format;
mod pac;

mod sealed {
    pub trait Sealed {}
}
//use self::sealed::Sealed;

//use core::marker::PhantomData;

pub use self::config::{MasterClock, MasterConfig, SlaveConfig};
use self::pac::spi1::RegisterBlock;
//use crate::format::{DataFormat, FrameFormat, FrameSync};
//use crate::pac::spi1::i2scfgr::I2SCFG_A;

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

/// An object composed of a SPI device that can be used for I2S communication.
///
/// This trait is meant to be implemented on a type that represent a full SPI device, that means an
/// object composed of a SPI peripheral, pins used by it, and eventually a clock object (can be a
/// reference).
///
/// # Safety
///
/// It is only safe to implement this trait when:
///
/// * The implementing type has ownership of the peripheral, preventing any other accesses to the
///   register block.
/// * `REGISTERS` is a pointer to that peripheral's register block and can be safely accessed  as
///   long as ownership or a borrow of the implementing type is present.
pub unsafe trait I2sPeripheral {
    /// Pointer to the SPI register block
    const REGISTERS: *const ();
}

/// Driver of a SPI peripheral in I2S mode
///
/// # Example
///
/// TODO
///
/// ```no_run
/// ```
///
pub struct I2sDriver<I> {
    _instance: I,
}

impl<I> I2sDriver<I>
where
    I: I2sPeripheral,
{
    /// Returns a reference to the register block
    fn registers(&self) -> &RegisterBlock {
        unsafe { &*(I::REGISTERS as *const RegisterBlock) }
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

impl<I> I2sDriver<I>
where
    I: I2sPeripheral,
{
    /// Enable the I2S peripheral.
    pub fn enable(&mut self) {
        self.registers().i2scfgr.modify(|_, w| w.i2se().enabled());
    }

    /// Immediately Disable the I2S peripheral.
    ///
    /// It's up to the caller to not disable the peripheral in the middle of a frame.
    pub fn disable(&mut self) {
        self.registers().i2scfgr.modify(|_, w| w.i2se().disabled());
    }

    /// Write a raw half word to the Tx buffer and delete the TXE flag in status register.
    ///
    /// It's up to the caller to write the content when it's empty.
    pub fn write_data_register(&mut self, value: u16) {
        self.registers().dr.write(|w| w.dr().bits(value));
    }

    /// Read a raw value from the Rx buffer and delete the RXNE flag in status register.
    pub fn read_data_register(&mut self) -> u16 {
        self.registers().dr.read().dr().bits()
    }

    /// When set to `true`, an interrupt is generated each time the Tx buffer is empty.
    pub fn set_tx_interrupt(&mut self, enabled: bool) {
        self.registers().cr2.modify(|_, w| w.txeie().bit(enabled))
    }

    /// When set to `true`, an interrupt is generated each time the Rx buffer contains a new data.
    pub fn set_rx_interrupt(&mut self, enabled: bool) {
        self.registers().cr2.modify(|_, w| w.rxneie().bit(enabled))
    }

    /// When set to `true`, an interrupt is generated each time an error occurs.
    pub fn set_error_interrupt(&mut self, enabled: bool) {
        self.registers().cr2.modify(|_, w| w.errie().bit(enabled))
    }

    /// When set to `true`, a dma request is generated each time the Tx buffer is empty.
    pub fn set_tx_dma(&mut self, enabled: bool) {
        self.registers().cr2.modify(|_, w| w.txdmaen().bit(enabled))
    }

    /// When set to `true`, a dma request is generated each time the Rx buffer contains a new data.
    pub fn set_rx_dma(&mut self, enabled: bool) {
        self.registers().cr2.modify(|_, w| w.rxdmaen().bit(enabled))
    }
}
