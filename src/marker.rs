//! Markers for [`Config`](super::Config) and [`I2sDriver`](super::I2sDriver)
use core::marker::PhantomData;

use crate::sealed::Sealed;

/// Marker, indicate operation mode of the I2sDriver.
///
///  - `MS`: `Master` or `Slave`
///  - `TR`: `Transmit` or `Receive`
#[derive(Debug, Clone, Copy)]
pub struct Mode<MS, TR> {
    _ms: PhantomData<MS>,
    _tr: PhantomData<TR>,
}

/// Marker, indicated master mode.
#[derive(Debug, Clone, Copy)]
pub struct Master;

/// Marker, indicate slave mode.
#[derive(Debug, Clone, Copy)]
pub struct Slave;

/// Marker, indicated transmit mode.
#[derive(Debug, Clone, Copy)]
pub struct Transmit;

/// Marker, indicate receive mode.
#[derive(Debug, Clone, Copy)]
pub struct Receive;

/// Marker, indicate 16 bits data length on 16 bits wide channel.
#[derive(Debug, Clone, Copy)]
pub struct Data16Channel16;

/// Marker, indicate 16 bits data length on 32 bits wide channel.
#[derive(Debug, Clone, Copy)]
pub struct Data16Channel32;

/// Marker, indicate 24 bits data length on 32 bits wide channel.
#[derive(Debug, Clone, Copy)]
pub struct Data24Channel32;

/// Marker, indicate 32 bits data length on 32 bits wide channel.
#[derive(Debug, Clone, Copy)]
pub struct Data32Channel32;

impl Sealed for Master {}
impl Sealed for Slave {}
impl Sealed for Transmit {}
impl Sealed for Receive {}
impl Sealed for Data16Channel16 {}
impl Sealed for Data16Channel32 {}
impl Sealed for Data24Channel32 {}
impl Sealed for Data32Channel32 {}

/// Trait for marker indicating 16 bits data length, that is `Data16Channel16` and
/// `Data16Channel32`
pub trait Data16: Sealed {}
impl Data16 for Data16Channel16 {}
impl Data16 for Data16Channel32 {}

/// Trait for marker indicating a DataFormat
pub trait DataFormat: Sealed {
    /// Runtime value.
    const VALUE: crate::DataFormat;
}

macro_rules! impl_data_format{
    ($($marker:ident),*) => {
        $(
            impl DataFormat for $marker {
                const VALUE: crate::DataFormat = crate::DataFormat::$marker;
            }
        )*
    };
}

impl_data_format!(
    Data16Channel16,
    Data16Channel32,
    Data24Channel32,
    Data32Channel32
);
