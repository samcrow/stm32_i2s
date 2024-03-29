//! This library supports I2S communication for SPI version 1.2 of STM32 microcontrollers (STM32F1,
//! STM32F2, STM32F4, STM32L0, and STM32L1). This library may also work with SPI version 1.3
//! (STM32F0 STM32F3 STM32F7 STM32L4 STM32L5)
//!
//! This library is normally used through a MCU HAL library providing types that implement
//! [`I2sPeripheral`] or [`DualI2sPeripheral`]. [`I2sDriver`](driver::I2sDriver) or
//! [`I2sTransfer`](transfer::I2sTransfer) objects can be created around I2sPeripheral objects to
//! have a single bus for I2S communication, and [`DualI2sDriver`](driver::DualI2sDriver) objects can
//! be created around DualI2sPeripheral for full duplex I2S
//! communication.
//!
//! # For STM32 MCU HAL implementers
//!
//! To support I2S by using this library, HAL implementers must implements [`I2sPeripheral`],
//! [`DualI2sPeripheral`] and [`WsPin`] and  trait and reexport this crate. It's also recommended
//! to create some examples. For reference, implementation and examples are (or will be soon)
//! available in [stm32f4xx-hal](https://github.com/stm32-rs/stm32f4xx-hal/).
//!
//! # For I2S users
//!
//! You should use use this library through a MCU HAL. For fine control and advanced usage,
//! see the [driver] module. For quick and basic usage, see the [transfer] module.
//!
//! # About PCM standards
//!
//! Almost all information you can get about PCM mode in STM32 datasheets are wrong, or confusing
//! at least. Compared to other modes:
//!  - PCM is monophonic; this is why the Channel flag information is meaningless.
//!  - With the same prescaler configuration, the sampling frequency is doubled. This is because
//!  the bit rate is the same with half the samples.
//!  - When master clock is enabled, its frequency is 128 * sampling_frequency, instead of 256 *
//!  sampling_frequency.
//!
//! # Issues and limitations
//! - In master transmit mode, the CHSIDE flag appears to be sporadically wrong, so don't use it.
//! - Depending on your chip, data corruption may occur under some configurations. Check the
//!   errata of your chip for more information.
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
/// This trait is meant to be implemented on a type that represent a full SPI device. That means an
/// object composed of a SPI peripheral, pins that it uses, and eventually a clock object (which can be a
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
    type WsPin: WsPin;
    /// Pointer to the SPI register block
    const REGISTERS: *const ();
    /// Get I2s clock source frequency from the I2s device.
    ///
    /// Implementers are allowed to panic in case i2s source frequency is unavailable.
    fn i2s_freq(&self) -> u32;
    /// Get a reference to WS pin.
    fn ws_pin(&self) -> &Self::WsPin;
    /// Get mutable reference to WS pin;
    fn ws_pin_mut(&mut self) -> &mut Self::WsPin;
    /// Reset the peripheral through the rcc register. This must be implemented with atomic
    /// operations through writes to the bit band region.
    fn rcc_reset(&mut self);
}

/// An object that can be used for full duplex I2S communication.
///
/// This trait is meant to be implemented on a type that represent a device supporting full duplex
/// I2S operation. This object should be composed of
///  - A SPI peripheral with I2S support
///  - The corresponding I2SEXT peripheral
///  - Pins that the peripherals use
///  - Eventually a clock object (or reference)
///
/// # Safety
///
/// It is only safe to implement this trait when:
///
/// * The implementing type has ownership of the peripherals, preventing any other accesses to the
/// register blocks.
/// * `MAIN_REGISTERS` and `EXT_REGISTERS` are pointers to that peripheral's register blocks and
/// can be safely accessed  as long as ownership or a borrow of the implementing type is present.
pub unsafe trait DualI2sPeripheral {
    type WsPin: WsPin;
    /// Pointer to the SPI register block
    const MAIN_REGISTERS: *const ();
    /// Pointer to the I2SEXT register block
    const EXT_REGISTERS: *const ();
    /// Get I2s clock source frequency from the I2s device.
    ///
    /// Implementers are allowed to panic in case i2s source frequency is unavailable.
    fn i2s_freq(&self) -> u32;
    /// Get a reference to WS pin.
    fn ws_pin(&self) -> &Self::WsPin;
    /// Get mutable reference to WS pin;
    fn ws_pin_mut(&mut self) -> &mut Self::WsPin;
    /// Reset the peripheral through the rcc register. This must be implemented with atomic
    /// operation through write to bit band region.
    fn rcc_reset(&mut self);
}

/// A pin carrying the WS (word select) signal from/to an i2s peripheral.
///
/// Implementing this trait means implementing read operation on a pin physically configured in
/// alternate mode.
pub trait WsPin {
    /// Return `true` if the level at WS pin is low.
    fn is_low(&self) -> bool;
    /// Return `true` if the level at WS pin is high.
    fn is_high(&self) -> bool;
}
