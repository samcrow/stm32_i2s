//! SPI/I2S register definitions
//!
//! This module is based on register definitions from an STM32F4 model. It should be compatible
//! with all STM32F1, STM32F2, STM32F4, STM32L0, and STM32L1 devices.
//!

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(unused)]
use core::marker::PhantomData;
use core::ops::Deref;

pub mod generic;
#[doc = "Serial peripheral interface"]
pub mod spi1;

use self::generic::*;
