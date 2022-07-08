//! This library supports I2S communication for SPI version 1.2 (on STM32F1, STM32F2, STM32F4,
//! STM32L0, and STM32L1 microcontrollers).
//!
//! This library is normally used through a MCU HAL library providing types that implement
//! [`I2sPeripheral`]. [`I2sDriver`](driver::I2sDriver) or [`I2sTransfer`](transfer::I2sTransfer)
//! objects can be created around I2sPeripheral object and used for I2S communication.
//!
//! # For stm32 MCU HAL implementers
//!
//! To support I2s by using this library, HAL implementers must implements [`I2sPeripheral`] trait
//! and reexport this crate. It's also recommended to create some example. For reference,
//! implementation and examples are (or will be soon) available in stm32f4xx-hal.
//!
//! # For i2s users
//!
//! For fine control and advanced usage, look [driver] module. For quick and basic usage, look
//! [transfer] module.
//!
//! # Issues and limitations
//!
//!  - In master mode, there is currently no way to reset clock phase.
//!  - In master transmit mode, the CHSIDE flag appear to be sporadically wrong
//!
//! As consequence :
//!  - for driver in master transmit, once driver has been disabled, it's may impossible to
//!  reliably know what is the next part to transmit.
//!  - for driver in master receive, this information can be recovered using CHSIDE flag. However,
//!  this doesn't work with PCM standard.
//!  - Once a transfer in master transmit mode have been disabled, it will work incorrectly until next
//!  MCU reset.
//!  - master receive transfer is not implemented for PCM.
//!
//!
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
    type WsPin;
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
    /// Reset the peripheral through the rcc register.
    fn ws_pin(&self) -> &Self::WsPin;
    fn ws_pin_mut(&mut self) -> &mut Self::WsPin;
    fn rcc_reset(&mut self);
}

/// Pin of an i2s peripheral configured in `WS` alternate mode.
pub trait WsPin {
    fn is_low(&self) -> bool;
    fn is_high(&self) -> bool;
}
