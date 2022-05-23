//! This library supports I2S communication for SPI version 1.2 (on STM32F1, STM32F2, STM32F4,
//! STM32L0, and STM32L1 microcontrollers).
//!
//! This library is normally used with a HAL library that provides a type that implements
//! [I2sPeripheral](crate::I2sPeripheral). An [I2sDriver](crate::I2sDriver) object can be created around the I2sPeripheral
//! object and used for I2S.

#![no_std]

mod pac;

pub mod driver;
pub mod marker;
pub mod transfer;

mod sealed {
    pub trait Sealed {}
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
    /// Get I2s clock source frequency from the I2s device.
    ///
    /// Implementers are allowed to panic in case i2s source frequency is unavailable.
    fn i2s_freq(&self) -> u32;
    /// Return `true` if the level at WS pin is high.
    fn ws_is_high(&self) -> bool;
    /// Return `true` if the level at WS pin is low.
    fn ws_is_low(&self) -> bool;
}
