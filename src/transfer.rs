//! Abstraction to transfer I2S data.
//!
//! API of this module give abstractions allowing to transfer I2S audio data while hiding the
//! hardware details. This module also have basis to implement the upcoming embedded-hale I2s
//! trait. The job is mainly done by [`I2sTransfer`], a type that wrap an I2sPeripheral to control
//! it.
//!
//! At the moment, transfer is not implemented for 24 bits data and for PCM standard in master
//! receive mode.
//!
//! # Configure and instantiate transfer.
//!
//! [`I2sTransferConfig`] is used to create configuration of the i2s transfer:
//! ```no_run
//! # use stm32_i2s_v12x::transfer::*;
//! let transfer_config = I2sTransferConfig::new_master()
//!     .receive()
//!     .standard(Philips)
//!     .data_format(Data16Channel32)
//!     .master_clock(true)
//!     .request_frequency(48_000);
//! ```
//! Then you can instantiate the transfer around an `I2sPeripheral`:
//! ```ignore
//! // instantiate from configuration
//! let transfer = transfer_config.i2s_transfer(i2s_peripheral);
//!
//! // alternate way
//! let transfer = I2sTransfer::new(i2s_peripheral, transfer_config);
//! ```
//!
//! # Transmitting data
//!
//! Transmitting data can be done with `write_iter` (blocking API) or `write` (non-blocking API)
//!
//! ```ignore
//! // Full scale sine wave spanning 32 samples. With a 48 kHz sampling rate this give a 1500 Hz
//! // signal.
//! const SINE_1500: [i16; 32] = [
//!     0, 6392, 12539, 18204, 23169, 27244, 30272, 32137, 32767, 32137, 30272, 27244, 23169,
//!     18204, 12539, 6392, 0, -6392, -12539, -18204, -23169, -27244, -30272, -32137, -32767,
//!     -32137, -30272, -27244, -23169, -18204, -12539, -6392,
//! ];
//!
//! // Iterator generating audio data for 1 sec (at 48 kHz sampling rate)
//! let sine_1500_iter = SINE_1500
//!     .iter()
//!     .map(|&x| (x, x))
//!     .cycle()
//!     .take(48_000 as usize);
//!
//! // write_iter (blocking API)
//! i2s2_transfer.write_iter(sine_1500_iter.clone());
//!
//! // equivalent using write (non-blocking);
//! for sample in sine_1500_iter.clone() {
//!     block!(i2s3_transfer.write(sample)).ok();
//! }
//! ```
//! # Receiving data
//!
//! Receiving data can be done with `read_while` (blocking API) or `read` (non-blocking API).
//! ```ignore
//! // buffer to record 1 second  of 8 bit mono data at 48 kHz
//! let mut buf = [0u8; 48000];
//!
//! // peekable iterator
//! let mut buf_iter = buf.iter_mut().peekable();
//!
//! // take left channel data and convert it into 8 bit data (blocking)
//! transfer.read_while(|s| {
//!     if let Some(b) = buf_iter.next() {
//!         *b = (s.0 >> 8) as u8;
//!     }
//!     buf_iter.peek().is_some()
//! });
//!
//! // equivalent with using read (non-blocking API)
//! for sample in sine_1500_iter.clone() {
//!     if let Some((l,_)) = block!(i2s3_transfer.read()) {
//!         *sample = (l >> 8) as u8;
//!     }
//! }
//! ```
//!
//! # Transmit and receive at same time
//!
//! The non-blocking API allow to process transmitting and receiving at same time. However, the
//! following example require two transfer using same clocks to work correctly:
//! ```ignore
//! let mut samples = (0, 0);
//! loop {
//!     if let Ok(s) = transfer1.read() {
//!         /* do some processing on s */
//!         samples = s;
//!     }
//!     transfer2.write(samples).ok();
//! }
//! ```
use crate::sealed::Sealed;
use core::convert::Infallible;
use core::marker::PhantomData;

use nb::Error::WouldBlock;

use crate::driver::ClockPolarity;
use crate::driver::I2sDriver as Driver;
use crate::driver::I2sDriverConfig as DriverConfig;
use crate::{I2sPeripheral, WsPin};

pub use crate::marker::{self, *};

/// trait to build Frame representation from markers.
pub trait FrameFormat: Sealed {
    /// raw frame representation for transfer implmentation, actual type is always an array of u16
    type RawFrame: Default + Copy + Sync + Send + AsRef<[u16]> + AsMut<[u16]>;
}

/// Syntax sugar to get appropriate raw frame representation;
pub type RawFrame<STD, FMT> = <(STD, FMT) as FrameFormat>::RawFrame;

macro_rules! impl_frame_format{
    ($(([$($std:ident),*],$fmt:ident,$raw_frame:ty)),*) => {
        $(
            $(
                impl FrameFormat for ($std,$fmt) {
                    type RawFrame = $raw_frame;
                }
            )*
        )*
    };
}

impl<T: Sealed, U: Sealed> Sealed for (T, U) {}

impl_frame_format!(
    ([Philips, Msb, Lsb], Data16Channel16, [u16; 2]),
    ([Philips, Msb, Lsb], Data16Channel32, [u16; 2]),
    ([Philips, Msb, Lsb], Data32Channel32, [u16; 4]),
    ([PcmShortSync, PcmLongSync], Data16Channel16, [u16; 1]),
    ([PcmShortSync, PcmLongSync], Data16Channel32, [u16; 1]),
    ([PcmShortSync, PcmLongSync], Data32Channel32, [u16; 2])
);

#[derive(Debug, Clone, Copy)]
/// [`I2sTransfer`] configuration.
///
///  - `MS`: `Master` or `Slave`
///  - `TR`: `Transmit` or `Receive`
///  - `STD`: I2S standard, eg `Philips`
///  - `FMT`: Frame Format marker, eg `Data16Channel16`
///
/// **Note:** because of it's typestate, methods of this type don't change variable content, they
/// return a new value instead.
pub struct I2sTransferConfig<MS, TR, STD, FMT> {
    driver_config: DriverConfig<MS, TR, STD>,
    _fmt: PhantomData<FMT>,
}

impl I2sTransferConfig<Slave, Transmit, Philips, Data16Channel16> {
    /// Create a new default slave configuration.
    pub fn new_slave() -> Self {
        Self {
            driver_config: DriverConfig::new_slave(),
            _fmt: PhantomData,
        }
    }
}

impl I2sTransferConfig<Master, Transmit, Philips, Data16Channel16> {
    /// Create a new default master configuration.
    pub fn new_master() -> Self {
        Self {
            driver_config: DriverConfig::new_master(),
            _fmt: PhantomData,
        }
    }
}

impl<MS, TR, STD, FMT> I2sTransferConfig<MS, TR, STD, FMT>
where
    STD: I2sStandard,
    FMT: DataFormat,
    (STD, FMT): FrameFormat,
{
    /// Create a `I2sTransfer` object around an [`I2sPeripheral`] object.
    ///
    /// # Panics
    ///
    /// This method panics if an exact frequency is required  and that frequency can not be set.
    pub fn i2s_transfer<I: I2sPeripheral>(
        self,
        i2s_peripheral: I,
    ) -> I2sTransfer<I, MS, TR, STD, FMT> {
        let driver = self.driver_config.i2s_driver(i2s_peripheral);
        I2sTransfer::<I, MS, TR, STD, FMT> {
            driver,
            frame: Default::default(),
            transfer_count: 0,
            sync: false,
            _fmt: PhantomData,
        }
    }
}

impl Default for I2sTransferConfig<Slave, Transmit, Philips, Data16Channel16> {
    /// Create a default configuration. It correspond to a default slave configuration.
    fn default() -> Self {
        Self::new_slave()
    }
}

impl<MS, TR, STD, FMT> I2sTransferConfig<MS, TR, STD, FMT> {
    /// Configure transfert for transmitting data.
    pub fn transmit(self) -> I2sTransferConfig<MS, Transmit, STD, FMT> {
        I2sTransferConfig::<MS, Transmit, STD, FMT> {
            driver_config: self.driver_config.transmit(),
            _fmt: PhantomData,
        }
    }
    /// Configure transfer for receiving data.
    pub fn receive(self) -> I2sTransferConfig<MS, Receive, STD, FMT> {
        I2sTransferConfig::<MS, Receive, STD, FMT> {
            driver_config: self.driver_config.receive(),
            _fmt: PhantomData,
        }
    }
    /// Select the I2s standard to use. The parameter is just a marker implementing [`I2sStandard`].
    #[allow(non_camel_case_types)]
    pub fn standard<NEW_STD>(self, _standard: NEW_STD) -> I2sTransferConfig<MS, TR, NEW_STD, FMT>
    where
        NEW_STD: marker::I2sStandard,
    {
        I2sTransferConfig::<MS, TR, NEW_STD, FMT> {
            driver_config: self.driver_config.standard(_standard),
            _fmt: PhantomData,
        }
    }
    /// Select steady state clock polarity
    pub fn clock_polarity(self, polarity: ClockPolarity) -> Self {
        I2sTransferConfig::<MS, TR, STD, FMT> {
            driver_config: self.driver_config.clock_polarity(polarity),
            _fmt: PhantomData,
        }
    }

    /// Select data format. The parameter is just a marker implementing [`DataFormat`].
    #[allow(non_camel_case_types)]
    pub fn data_format<NEW_FMT>(self, _format: NEW_FMT) -> I2sTransferConfig<MS, TR, STD, NEW_FMT>
    where
        NEW_FMT: marker::DataFormat,
    {
        I2sTransferConfig::<MS, TR, STD, NEW_FMT> {
            driver_config: self.driver_config.data_format(NEW_FMT::VALUE),
            _fmt: PhantomData,
        }
    }

    /// Convert to a slave configuration. This delete Master Only Settings.
    pub fn to_slave(self) -> I2sTransferConfig<Slave, TR, STD, FMT> {
        I2sTransferConfig::<Slave, TR, STD, FMT> {
            driver_config: self.driver_config.to_slave(),
            _fmt: PhantomData,
        }
    }

    /// Convert to a master configuration.
    pub fn to_master(self) -> I2sTransferConfig<Master, TR, STD, FMT> {
        I2sTransferConfig::<Master, TR, STD, FMT> {
            driver_config: self.driver_config.to_master(),
            _fmt: PhantomData,
        }
    }
}

impl<TR, STD, FMT> I2sTransferConfig<Master, TR, STD, FMT> {
    /// Enable/Disable Master Clock. Affect the effective sampling rate.
    ///
    /// This can be only set and only have meaning for Master mode.
    pub fn master_clock(self, enable: bool) -> Self {
        I2sTransferConfig::<Master, TR, STD, FMT> {
            driver_config: self.driver_config.master_clock(enable),
            _fmt: PhantomData,
        }
    }

    /// Configure audio frequency of the transfer by setting the prescaler with an odd factor and a
    /// divider.
    ///
    /// The effective sampling frequency is:
    ///  - `i2s_clock / [256 * ((2 * div) + odd)]` when master clock is enabled
    ///  - `i2s_clock / [(channel_length * 2) * ((2 * div) + odd)]` when master clock is disabled
    ///
    ///  `i2s_clock` is I2S clock source frequency, and `channel_length` is width in bits of the
    ///  channel (see [DataFormat])
    ///
    /// This setting only have meaning and can be only set for master.
    ///
    /// # Panics
    ///
    /// `div` must be at least 2, otherwise the method panics.
    pub fn prescaler(self, odd: bool, div: u8) -> Self {
        I2sTransferConfig::<Master, TR, STD, FMT> {
            driver_config: self.driver_config.prescaler(odd, div),
            _fmt: PhantomData,
        }
    }

    /// Request an audio sampling frequency. The effective audio sampling frequency may differ.
    pub fn request_frequency(self, freq: u32) -> Self {
        I2sTransferConfig::<Master, TR, STD, FMT> {
            driver_config: self.driver_config.request_frequency(freq),
            _fmt: PhantomData,
        }
    }

    /// Require exactly this audio sampling frequency.
    ///
    /// If the required frequency can not bet set, Instantiate a transfer will panics.
    pub fn require_frequency(self, freq: u32) -> Self {
        I2sTransferConfig::<Master, TR, STD, FMT> {
            driver_config: self.driver_config.require_frequency(freq),
            _fmt: PhantomData,
        }
    }
}

/// Part of the frame we currently transmitting or receiving
#[derive(Debug, Clone, Copy)]
enum FrameState {
    LeftMsb,
    LeftLsb,
    RightMsb,
    RightLsb,
}
use FrameState::*;

/// Abstraction allowing to transmit/receive I2S data while erasing hardware details.
///
/// This type is meant to implement the Upcoming I2S embbeded-hal in the future.
///
/// Note: current implementation never fail when an error is detected, it try to recover intead. As
/// result, data received or transmitted may corrupted. This choice has been made because:
///  - corrupted data can't produce an invalid value that can cause undefined behavior,
///  - audio quality is equally degraded by missing or corrupted data,
///  - it's easier to use.
pub struct I2sTransfer<I, MS, TR, STD, FMT>
where
    I: I2sPeripheral,
    (STD, FMT): FrameFormat,
{
    driver: Driver<I, MS, TR, STD>,
    frame: RawFrame<STD, FMT>,
    transfer_count: u8, //track part of the frame we transmitting
    sync: bool,
    _fmt: PhantomData<FMT>,
}

impl<I, MS, TR, STD, FMT> I2sTransfer<I, MS, TR, STD, FMT>
where
    I: I2sPeripheral,
    STD: I2sStandard,
    (STD, FMT): FrameFormat,
{
    /// When `true` the level on WS line make start a slave. The slave must be enabled before this
    /// level is set.
    #[inline]
    fn _ws_is_start(&self) -> bool {
        match STD::WS_START_LEVEL {
            false => self.driver.ws_pin().is_low(),
            true => self.driver.ws_pin().is_high(),
        }
    }
}

/// Constructors and Destructors
impl<I, MS, TR, STD, FMT> I2sTransfer<I, MS, TR, STD, FMT>
where
    I: I2sPeripheral,
    STD: I2sStandard,
    FMT: DataFormat,
    (STD, FMT): FrameFormat,
{
    /// Instantiate and configure an i2s driver around an [`I2sPeripheral`].
    ///
    /// # Panics
    ///
    /// This method panics if an exact frequency is required by the config and that frequency can
    /// not be set.
    pub fn new(i2s_peripheral: I, config: I2sTransferConfig<MS, TR, STD, FMT>) -> Self {
        config.i2s_transfer(i2s_peripheral)
    }

    /// Destroy the transfer, release the owned i2s device and reset it's configuration.
    pub fn release(self) -> I {
        self.driver.release()
    }
}

impl<I, MS, TR, STD, FMT> I2sTransfer<I, MS, TR, STD, FMT>
where
    I: I2sPeripheral,
    (STD, FMT): FrameFormat,
{
    /// Activate the I2s interface.
    pub fn begin(&mut self) {
        self.driver.enable()
    }
}

impl<I, TR, STD, FMT> I2sTransfer<I, Slave, TR, STD, FMT>
where
    I: I2sPeripheral,
    (STD, FMT): FrameFormat,
{
    /// Deactivate the I2s interface and reset internal state
    pub fn end(&mut self) {
        self.driver.disable();
        self.frame = Default::default();
        self.transfer_count = 0;
        self.sync = false;
    }
}

impl<I, TR, STD, FMT> I2sTransfer<I, Master, TR, STD, FMT>
where
    I: I2sPeripheral,
    (STD, FMT): FrameFormat,
{
    /// Deactivate the I2s interface and reset internal state
    pub fn end(&mut self) {
        self.driver.disable();
        self.driver.reset_clocks();
        self.frame = Default::default();
        self.transfer_count = 0;
        self.sync = false;
    }
}

impl<I, TR, STD, FMT> I2sTransfer<I, Master, TR, STD, FMT>
where
    I: I2sPeripheral,
    (STD, FMT): FrameFormat,
{
    pub fn sample_rate(&self) -> u32 {
        self.driver.sample_rate()
    }
}
