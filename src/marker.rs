//! Markers for [`I2sDriver`](crate::driver::I2sDriver),
//! [`I2sTransfer`](crate::transfer::I2sTransfer) and their respective configuration.
use crate::sealed::Sealed;

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

/// Marker, indicate Philips I2S standard.
#[derive(Debug, Clone, Copy)]
pub struct Philips;

/// Marker, indicate MSB Justified standard.
#[derive(Debug, Clone, Copy)]
pub struct Msb;

/// Marker, indicate LSB Justified standard.
#[derive(Debug, Clone, Copy)]
pub struct Lsb;

/// Marker, indicate PCM standard with short frame synchronisation.
#[derive(Debug, Clone, Copy)]
pub struct PcmShortSync;

/// Marker, indicate PCM standard with long frame synchronisation.
#[derive(Debug, Clone, Copy)]
pub struct PcmLongSync;

/// Marker, indicate main part of a dual i2s device.
#[derive(Debug, Clone, Copy)]
pub struct Main;

/// Marker, indicate extension part of a dual i2s device.
#[derive(Debug, Clone, Copy)]
pub struct Ext;

impl Sealed for Master {}
impl Sealed for Slave {}
impl Sealed for Transmit {}
impl Sealed for Receive {}
impl Sealed for Data16Channel16 {}
impl Sealed for Data16Channel32 {}
impl Sealed for Data24Channel32 {}
impl Sealed for Data32Channel32 {}
impl Sealed for Philips {}
impl Sealed for Msb {}
impl Sealed for Lsb {}
impl Sealed for PcmShortSync {}
impl Sealed for PcmLongSync {}
impl Sealed for Main {}
impl Sealed for Ext {}

/// Trait for `Transmit` and `Receive` markers.
pub trait Direction: Sealed {
    /// Internal use only (used by configuration types).
    const VALUE: crate::driver::TransmitOrReceive;
}
impl Direction for Transmit {
    const VALUE: crate::driver::TransmitOrReceive = crate::driver::TransmitOrReceive::Transmit;
}
impl Direction for Receive {
    const VALUE: crate::driver::TransmitOrReceive = crate::driver::TransmitOrReceive::Receive;
}

/// Trait for marker indicating 16 bits data length, that is `Data16Channel16` and
/// `Data16Channel32`
pub trait Data16: Sealed {}
impl Data16 for Data16Channel16 {}
impl Data16 for Data16Channel32 {}

/// Trait for marker indicating a DataFormat
pub trait DataFormat: Sealed {
    /// Internal use only (used by configuration types).
    const VALUE: crate::driver::DataFormat;
    /// Audio frame representation from API point of view;
    type AudioFrame: Default;
}

macro_rules! impl_data_format{
    ($(($marker:ident,$audio_frame:ty)),*) => {
        $(
            impl DataFormat for $marker {
                const VALUE: crate::driver::DataFormat = crate::driver::DataFormat::$marker;
                type AudioFrame = $audio_frame;
            }
        )*
    };
}

impl_data_format!(
    (Data16Channel16, (i16, i16)),
    (Data16Channel32, (i16, i16)),
    (Data24Channel32, (i32, i32)),
    (Data32Channel32, (i32, i32))
);

/// Trait for marker indicating a i2s standard.
pub trait I2sStandard: Sealed {
    /// Internal use only (used by configuration types).
    const VALUE: crate::driver::I2sStandard;
    /// WS line level that make start the i2s device. `true` mean high level.
    ///
    /// Slave need to be enabled when WS line is **not** at this level.
    const WS_START_LEVEL: bool;
}

macro_rules! impl_i2s_standard{
    ($(($marker:ident,$ws_start_level:literal)),*) => {
        $(
            impl I2sStandard for $marker {
                const VALUE: crate::driver::I2sStandard = crate::driver::I2sStandard::$marker;
                const WS_START_LEVEL: bool = $ws_start_level;
            }
        )*
    };
}

impl_i2s_standard!(
    (Philips, false),
    (Msb, true),
    (Lsb, true),
    (PcmShortSync, true),
    (PcmLongSync, true)
);

/// Marker trait for i2s standard where the CHSIDE status flag is a relevant channel indication
/// about data been received or to be transmitted.
pub trait ChannelFlag: I2sStandard + Sealed {}

impl ChannelFlag for Philips {}
impl ChannelFlag for Msb {}
impl ChannelFlag for Lsb {}

/// Marker trait for i2s standard where the CHSIDE status flag is meaningless.
pub trait NoChannelFlag: I2sStandard + Sealed {}

impl NoChannelFlag for PcmShortSync {}
impl NoChannelFlag for PcmLongSync {}
