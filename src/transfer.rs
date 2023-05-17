//! Abstraction to transfer I2S data.
//!
//! The API of this module allows transferring I2S audio data while hiding the
//! hardware details. This module also is a basis for implementations of the upcoming embedded-hal I2s
//! trait. The job is mainly done by [`I2sTransfer`], a type that wraps an I2sPeripheral to control
//! it.
//!
//! At the moment, transfer is not implemented for 24-bit data.
//!
//! # Configure and instantiate transfer
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
//! let mut transfer = transfer_config.i2s_transfer(i2s_peripheral);
//!
//! // alternate way
//! let mut transfer = I2sTransfer::new(i2s_peripheral, transfer_config);
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
//! let sine_1500_iter = SINE_1500.iter().map(|&x| (x, x)).cycle().take(48_000);
//!
//! // write_iter (blocking API)
//! transfer.write_iter(sine_1500_iter.clone());
//!
//! // equivalent using write (non-blocking);
//! for sample in sine_1500_iter.clone() {
//!     block!(transfer.write(sample)).ok();
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
//! transfer.read_while(|s: (i16, i16)| {
//!     if let Some(b) = buf_iter.next() {
//!         *b = (s.0 >> 8) as u8;
//!     }
//!     buf_iter.peek().is_some()
//! });
//!
//! // equivalent with using read (non-blocking API)
//! for s in buf.iter_mut() {
//!     if let Ok((l, _)) = block!(transfer.read()) {
//!         *s = (l >> 8) as u8;
//!     }
//! }
//! ```
//!
//! # Transmit and receive at the same time
//!
//! The non-blocking API allows transmitting and receiving at the same time. However, the
//! following example requires that both transfers use the same clocks to work correctly:
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

/// Trait to build an internal frame representation of an `I2sTransfer` from markers.
#[doc(hidden)]
pub trait FrameFormat: Sealed {
    /// Raw frame representation for transfer implementation
    ///
    /// The actual type is always an array of u16
    type RawFrame: Default + Copy + Sync + Send + AsRef<[u16]> + AsMut<[u16]>;
}

/// Syntax sugar to get the appropriate internal frame representation from markers.
type RawFrame<STD, FMT> = <(STD, FMT) as FrameFormat>::RawFrame;

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

/// Types written to `I2sTransfer`.
pub trait ToRawFrame<STD, FMT>
where
    (STD, FMT): FrameFormat,
{
    fn to_raw(&self) -> RawFrame<STD, FMT>;
}

macro_rules! impl_to_raw_frame{
    ($(($type:ty,[$($std:ident),*],$fmt:ident),$func:item),*) => {
        $(
            $(
                impl ToRawFrame<$std, $fmt> for $type {
                    $func
                }
            )*
        )*
    };
}

impl_to_raw_frame!(
    ((i16, i16), [Philips, Msb, Lsb], Data16Channel16),
    fn to_raw(&self) -> [u16; 2] {
        [self.0 as u16, self.1 as u16]
    },
    ((i16, i16), [Philips, Msb, Lsb], Data16Channel32),
    fn to_raw(&self) -> [u16; 2] {
        [self.0 as u16, self.1 as u16]
    },
    ((i32, i32), [Philips, Msb, Lsb], Data32Channel32),
    fn to_raw(&self) -> [u16; 4] {
        [
            (self.0 as u32 >> 16) as u16,
            (self.0 as u32 & 0xFFFF) as u16,
            (self.1 as u32 >> 16) as u16,
            (self.1 as u32 & 0xFFFF) as u16,
        ]
    },
    (i16, [PcmShortSync, PcmLongSync], Data16Channel16),
    fn to_raw(&self) -> [u16; 1] {
        [*self as u16]
    },
    (i16, [PcmShortSync, PcmLongSync], Data16Channel32),
    fn to_raw(&self) -> [u16; 1] {
        [*self as u16]
    },
    (i32, [PcmShortSync, PcmLongSync], Data32Channel32),
    fn to_raw(&self) -> [u16; 2] {
        [(*self as u32 >> 16) as u16, (*self as u32 & 0xFFFF) as u16]
    }
);

/// Types read from `I2sTransfer`.
pub trait FromRawFrame<STD, FMT>
where
    (STD, FMT): FrameFormat,
{
    fn from_raw(raw: RawFrame<STD, FMT>) -> Self;
}

macro_rules! impl_from_raw_frame{
    ($(($type:ty,[$($std:ident),*],$fmt:ident),$func:item),*) => {
        $(
            $(
                impl FromRawFrame<$std, $fmt> for $type {
                    $func
                }
            )*
        )*
    };
}

impl_from_raw_frame!(
    ((i16, i16), [Philips, Msb, Lsb], Data16Channel16),
    fn from_raw(raw: [u16; 2]) -> Self {
        (raw[0] as i16, raw[1] as i16)
    },
    ((i16, i16), [Philips, Msb, Lsb], Data16Channel32),
    fn from_raw(raw: [u16; 2]) -> Self {
        (raw[0] as i16, raw[1] as i16)
    },
    ((i32, i32), [Philips, Msb, Lsb], Data32Channel32),
    fn from_raw(raw: [u16; 4]) -> Self {
        let l = (raw[0] as i32) << 16 | raw[1] as i32;
        let r = (raw[2] as i32) << 16 | raw[3] as i32;
        (l, r)
    },
    (i16, [PcmShortSync, PcmLongSync], Data16Channel16),
    fn from_raw(raw: [u16; 1]) -> Self {
        raw[0] as i16
    },
    (i16, [PcmShortSync, PcmLongSync], Data16Channel32),
    fn from_raw(raw: [u16; 1]) -> Self {
        raw[0] as i16
    },
    (i32, [PcmShortSync, PcmLongSync], Data32Channel32),
    fn from_raw(raw: [u16; 2]) -> Self {
        (raw[0] as i32) << 16 | raw[1] as i32
    }
);

/// Errors that may require a special handling.
#[non_exhaustive]
pub enum I2sTransferError {
    Overrun,
}

#[derive(Debug, Clone, Copy)]
/// [`I2sTransfer`] configuration.
///
///  - `MS`: `Master` or `Slave`
///  - `DIR`: `Transmit` or `Receive`
///  - `STD`: I2S standard, eg `Philips`
///  - `FMT`: Frame Format marker, eg `Data16Channel16`
///
/// **Note:** because of its typestate, methods of this type don't modify a config object. They
/// return a new object instead.
pub struct I2sTransferConfig<MS, DIR, STD, FMT> {
    driver_config: DriverConfig<MS, DIR, STD>,
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

impl<MS, DIR, STD, FMT> I2sTransferConfig<MS, DIR, STD, FMT>
where
    STD: I2sStandard,
    FMT: DataFormat,
    (STD, FMT): FrameFormat,
{
    /// Create a `I2sTransfer` object around an [`I2sPeripheral`] object.
    ///
    /// # Panics
    ///
    /// This method panics if an exact frequency is required and that frequency can not be set.
    pub fn i2s_transfer<I: I2sPeripheral>(
        self,
        i2s_peripheral: I,
    ) -> I2sTransfer<I, MS, DIR, STD, FMT> {
        let driver = self.driver_config.i2s_driver(i2s_peripheral);
        I2sTransfer::<I, MS, DIR, STD, FMT> {
            driver,
            frame: Default::default(),
            transfer_count: 0,
            sync: false,
            _fmt: PhantomData,
        }
    }
}

impl Default for I2sTransferConfig<Slave, Transmit, Philips, Data16Channel16> {
    /// Create a default configuration. This corresponds to a default slave configuration.
    fn default() -> Self {
        Self::new_slave()
    }
}

impl<MS, DIR, STD, FMT> I2sTransferConfig<MS, DIR, STD, FMT> {
    /// Configure for transmitting data.
    pub fn transmit(self) -> I2sTransferConfig<MS, Transmit, STD, FMT> {
        I2sTransferConfig::<MS, Transmit, STD, FMT> {
            driver_config: self.driver_config.transmit(),
            _fmt: PhantomData,
        }
    }
    /// Configure for receiving data.
    pub fn receive(self) -> I2sTransferConfig<MS, Receive, STD, FMT> {
        I2sTransferConfig::<MS, Receive, STD, FMT> {
            driver_config: self.driver_config.receive(),
            _fmt: PhantomData,
        }
    }
    /// Select the I2s standard to use. The parameter is just a marker implementing [`I2sStandard`].
    #[allow(non_camel_case_types)]
    pub fn standard<NEW_STD>(self, _standard: NEW_STD) -> I2sTransferConfig<MS, DIR, NEW_STD, FMT>
    where
        NEW_STD: marker::I2sStandard,
    {
        I2sTransferConfig::<MS, DIR, NEW_STD, FMT> {
            driver_config: self.driver_config.standard(_standard),
            _fmt: PhantomData,
        }
    }
    /// Select steady state clock polarity
    pub fn clock_polarity(self, polarity: ClockPolarity) -> Self {
        I2sTransferConfig::<MS, DIR, STD, FMT> {
            driver_config: self.driver_config.clock_polarity(polarity),
            _fmt: PhantomData,
        }
    }

    /// Select data format. The parameter is just a marker implementing [`DataFormat`].
    #[allow(non_camel_case_types)]
    pub fn data_format<NEW_FMT>(self, _format: NEW_FMT) -> I2sTransferConfig<MS, DIR, STD, NEW_FMT>
    where
        NEW_FMT: marker::DataFormat,
    {
        I2sTransferConfig::<MS, DIR, STD, NEW_FMT> {
            driver_config: self.driver_config.data_format(NEW_FMT::VALUE),
            _fmt: PhantomData,
        }
    }

    /// Convert to a slave configuration.
    ///
    /// This deletes Master Only Settings.
    pub fn to_slave(self) -> I2sTransferConfig<Slave, DIR, STD, FMT> {
        I2sTransferConfig::<Slave, DIR, STD, FMT> {
            driver_config: self.driver_config.to_slave(),
            _fmt: PhantomData,
        }
    }

    /// Convert to a master configuration.
    pub fn to_master(self) -> I2sTransferConfig<Master, DIR, STD, FMT> {
        I2sTransferConfig::<Master, DIR, STD, FMT> {
            driver_config: self.driver_config.to_master(),
            _fmt: PhantomData,
        }
    }
}

impl<DIR, STD, FMT> I2sTransferConfig<Master, DIR, STD, FMT> {
    /// Enable/Disable Master Clock.
    ///
    /// This changes the effective sampling rate.
    ///
    /// This applies to Master mode only.
    pub fn master_clock(self, enable: bool) -> Self {
        I2sTransferConfig::<Master, DIR, STD, FMT> {
            driver_config: self.driver_config.master_clock(enable),
            _fmt: PhantomData,
        }
    }

    /// Configure audio sample rate of the transfer by setting the prescaler with an odd factor and a
    /// divider.
    ///
    /// The effective sampling frequency is:
    ///  - `i2s_clock / [256 * ((2 * div) + odd)]` when master clock is enabled
    ///  - `i2s_clock / [(channel_length * 2) * ((2 * div) + odd)]` when master clock is disabled
    ///
    ///  `i2s_clock` is I2S clock source frequency, and `channel_length` is width in bits of the
    ///  channel (see [DataFormat])
    ///
    /// This setting applies to Master mode only.
    ///
    /// # Panics
    ///
    /// `div` must be at least 2, otherwise the method panics.
    pub fn prescaler(self, odd: bool, div: u8) -> Self {
        I2sTransferConfig::<Master, DIR, STD, FMT> {
            driver_config: self.driver_config.prescaler(odd, div),
            _fmt: PhantomData,
        }
    }

    /// Request an audio sampling frequency. The effective audio sampling frequency may be different.
    pub fn request_frequency(self, freq: u32) -> Self {
        I2sTransferConfig::<Master, DIR, STD, FMT> {
            driver_config: self.driver_config.request_frequency(freq),
            _fmt: PhantomData,
        }
    }

    /// Require exactly this audio sampling frequency.
    ///
    /// If the required frequency can not bet set, instantiating a transfer will panic.
    pub fn require_frequency(self, freq: u32) -> Self {
        I2sTransferConfig::<Master, DIR, STD, FMT> {
            driver_config: self.driver_config.require_frequency(freq),
            _fmt: PhantomData,
        }
    }
}

/// Abstraction allowing sending and receiving of I2S data while erasing hardware details.
///
/// This type is meant to implement the upcoming embeded-hal I2S trait.
///
/// ## Implementation notes
///
/// `I2sTransfer` in slave mode never fails when an error is detected. Instead, it tries to recover
/// although some data may corrupted. This choice has been made because:
///  - corrupted data can't produce invalid audio values and therefore can't cause undefined
///  behavior,
///  - audio quality is equally degraded by missing or corrupted data,
///  - it's easier to use.
///
/// `I2sTransfer` in master receive mode fails when an overrun occurs. This is because `I2sTransfer`
/// resets clocks to recover and some parts of the peripheral need to be reset during this process.
///
///  `I2sTransfer` in master transmit never fails because the hardware can't detect errors in this
///  mode.
pub struct I2sTransfer<I, MS, DIR, STD, FMT>
where
    I: I2sPeripheral,
    (STD, FMT): FrameFormat,
{
    driver: Driver<I, MS, DIR, STD>,
    frame: RawFrame<STD, FMT>,
    transfer_count: u8, //track part of the frame we transmitting
    sync: bool,
    _fmt: PhantomData<FMT>,
}

impl<I, MS, DIR, STD, FMT> I2sTransfer<I, MS, DIR, STD, FMT>
where
    I: I2sPeripheral,
    STD: I2sStandard,
    (STD, FMT): FrameFormat,
{
    /// When `true`, the level on WS line is correct for the peripheral to start operating.
    ///
    /// The peripheral must be enabled before this level is set.
    #[inline]
    fn _ws_is_start(&self) -> bool {
        match STD::WS_START_LEVEL {
            false => self.driver.ws_pin().is_low(),
            true => self.driver.ws_pin().is_high(),
        }
    }
}

/// Constructors and Destructors
impl<I, MS, DIR, STD, FMT> I2sTransfer<I, MS, DIR, STD, FMT>
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
    /// This method panics if an exact frequency is required by the config and that frequency
    /// cannot be set.
    pub fn new(i2s_peripheral: I, config: I2sTransferConfig<MS, DIR, STD, FMT>) -> Self {
        config.i2s_transfer(i2s_peripheral)
    }

    /// Destroy the transfer, release the owned i2s device, and reset its configuration.
    pub fn release(self) -> I {
        self.driver.release()
    }
}

impl<I, MS, DIR, STD, FMT> I2sTransfer<I, MS, DIR, STD, FMT>
where
    I: I2sPeripheral,
    (STD, FMT): FrameFormat,
{
    /// Activate the I2s interface.
    pub fn begin(&mut self) {
        self.driver.enable()
    }
}

impl<I, DIR, STD, FMT> I2sTransfer<I, Slave, DIR, STD, FMT>
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

impl<I, DIR, STD, FMT> I2sTransfer<I, Master, DIR, STD, FMT>
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

impl<I, DIR, STD, FMT> I2sTransfer<I, Master, DIR, STD, FMT>
where
    I: I2sPeripheral,
    (STD, FMT): FrameFormat,
{
    pub fn sample_rate(&self) -> u32 {
        self.driver.sample_rate()
    }
}

/// Master Transmit
impl<I, STD, FMT> I2sTransfer<I, Master, Transmit, STD, FMT>
where
    I: I2sPeripheral,
    (STD, FMT): FrameFormat,
{
    /// Transmit (blocking) data from an iterator.
    pub fn write_iter<ITER, T>(&mut self, samples: ITER)
    where
        T: ToRawFrame<STD, FMT>,
        ITER: IntoIterator<Item = T>,
    {
        let mut samples = samples.into_iter();
        self.driver.enable();
        loop {
            let status = self.driver.status();
            if status.txe() {
                // having this check before give a chance to optimizer to remove bound checking on
                // array access
                if self.transfer_count >= self.frame.as_ref().len() as u8 {
                    self.transfer_count = 0;
                }
                if self.transfer_count == 0 {
                    let smpl = samples.next();
                    //breaking here ensure the last frame is fully transmitted
                    if smpl.is_none() {
                        break;
                    }
                    self.frame = smpl.unwrap().to_raw();
                }
                self.driver
                    .write_data_register(self.frame.as_ref()[self.transfer_count as usize]);
                self.transfer_count += 1;
            }
        }
    }

    /// Write one audio frame and activate the I2s interface if disabled.
    ///
    /// To fully transmit the frame, this function need to be continuously called until the next
    /// frame can be written.
    pub fn write<T: ToRawFrame<STD, FMT>>(&mut self, frame: T) -> nb::Result<(), Infallible> {
        self.driver.enable();
        let status = self.driver.status();
        if status.txe() {
            // having this check before give a chance to optimizer to remove bound checking on
            // array access
            if self.transfer_count >= self.frame.as_ref().len() as u8 {
                self.transfer_count = 0;
            }
            if self.transfer_count == 0 {
                self.frame = frame.to_raw();
                self.driver
                    .write_data_register(self.frame.as_ref()[self.transfer_count as usize]);
                self.transfer_count += 1;
                return Ok(());
            } else {
                self.driver
                    .write_data_register(self.frame.as_ref()[self.transfer_count as usize]);
                self.transfer_count += 1;
            }
        }
        Err(WouldBlock)
    }
}

/// Slave Transmit
impl<I, STD, FMT> I2sTransfer<I, Slave, Transmit, STD, FMT>
where
    I: I2sPeripheral,
    STD: I2sStandard,
    (STD, FMT): FrameFormat,
{
    /// Transmit (blocking) data from an iterator.
    pub fn write_iter<ITER, T>(&mut self, frames: ITER)
    where
        T: ToRawFrame<STD, FMT>,
        ITER: IntoIterator<Item = T>,
    {
        let mut frames = frames.into_iter();
        loop {
            if self.sync {
                let status = self.driver.status();
                if status.txe() {
                    // having this check before give a chance to optimizer to remove bound checking on
                    // array access
                    if self.transfer_count >= self.frame.as_ref().len() as u8 {
                        self.transfer_count = 0;
                    }
                    if self.transfer_count == 0 {
                        let frm = frames.next();
                        //breaking here ensure the last frame is fully transmitted
                        if frm.is_none() {
                            break;
                        }
                        self.frame = frm.unwrap().to_raw();
                    }
                    self.driver
                        .write_data_register(self.frame.as_ref()[self.transfer_count as usize]);
                    self.transfer_count += 1;
                }
                if status.fre() || status.udr() {
                    self.sync = false;
                    self.driver.disable();
                }
            } else if !self._ws_is_start() {
                // data register may (or not) already contain data, causing uncertainty about next
                // time txe flag is set. Writing it remove the uncertainty.
                let frm = frames.next();
                //breaking here ensure the last frame is fully transmitted
                if frm.is_none() {
                    break;
                }
                self.frame = frm.unwrap().to_raw();
                self.driver.write_data_register(self.frame.as_ref()[0]);
                self.transfer_count = 1;
                self.driver.enable();
                // ensure the ws line didn't change during sync process
                if !self._ws_is_start() {
                    self.sync = true;
                } else {
                    self.driver.disable();
                }
            }
        }
    }

    /// Write one audio frame and activate the I2s interface if disabled.
    ///
    /// To fully transmit the frame, this function need to be continuously called until the next
    /// frame can be written.
    pub fn write<T: ToRawFrame<STD, FMT>>(&mut self, frame: T) -> nb::Result<(), Infallible> {
        if self.sync {
            let status = self.driver.status();
            if status.txe() {
                // having this check before give a chance to optimizer to remove bound checking on
                // array access
                if self.transfer_count >= self.frame.as_ref().len() as u8 {
                    self.transfer_count = 0;
                }
                if self.transfer_count == 0 {
                    self.frame = frame.to_raw();
                    self.driver
                        .write_data_register(self.frame.as_ref()[self.transfer_count as usize]);
                    self.transfer_count += 1;
                    return Ok(());
                } else {
                    self.driver
                        .write_data_register(self.frame.as_ref()[self.transfer_count as usize]);
                    self.transfer_count += 1;
                }
            }
            if status.fre() || status.udr() {
                self.sync = false;
                self.driver.disable();
            }
        } else if !self._ws_is_start() {
            // data register may (or not) already contain data, causing uncertainty about next
            // time txe flag is set. Writing it remove the uncertainty.
            self.driver.write_data_register(self.frame.as_ref()[0]);
            self.transfer_count = 1;
            self.driver.enable();
            // ensure the ws line didn't change during sync process
            if !self._ws_is_start() {
                self.sync = true;
            } else {
                self.driver.disable();
            }
            return Ok(());
        }
        Err(WouldBlock)
    }
}

/// Master Receive
impl<I, STD, FMT> I2sTransfer<I, Master, Receive, STD, FMT>
where
    I: I2sPeripheral,
    (STD, FMT): FrameFormat,
{
    /// Read samples while predicate return `true`.
    ///
    /// The given closure must not block, otherwise communication problems may occur.
    pub fn read_while<F, T>(&mut self, mut predicate: F) -> Result<(), I2sTransferError>
    where
        T: FromRawFrame<STD, FMT>,
        F: FnMut(T) -> bool,
    {
        self.driver.enable();
        loop {
            let status = self.driver.status();
            if status.rxne() {
                if self.transfer_count >= self.frame.as_ref().len() as u8 {
                    self.transfer_count = 0;
                }
                self.frame.as_mut()[self.transfer_count as usize] =
                    self.driver.read_data_register();
                self.transfer_count += 1;

                // note: boolean operators are short-circuiting
                if self.transfer_count >= self.frame.as_ref().len() as u8
                    && !predicate(T::from_raw(self.frame))
                {
                    return Ok(());
                }
            }
            if status.ovr() {
                self.end();
                return Err(I2sTransferError::Overrun);
            }
        }
    }

    /// Read one audio frame and activate the I2s interface if disabled.
    ///
    /// To get the audio frame, this function needs to be continuously called until the frame is
    /// returned
    pub fn read<T: FromRawFrame<STD, FMT>>(&mut self) -> nb::Result<T, I2sTransferError> {
        self.driver.enable();
        let status = self.driver.status();
        if status.rxne() {
            if self.transfer_count >= self.frame.as_ref().len() as u8 {
                self.transfer_count = 0;
            }
            self.frame.as_mut()[self.transfer_count as usize] = self.driver.read_data_register();
            self.transfer_count += 1;

            if self.transfer_count >= self.frame.as_ref().len() as u8 {
                return Ok(T::from_raw(self.frame));
            }
        }
        if status.ovr() {
            self.end();
            return Err(nb::Error::Other(I2sTransferError::Overrun));
        }
        Err(WouldBlock)
    }
}

impl<I, STD, FMT> I2sTransfer<I, Slave, Receive, STD, FMT>
where
    I: I2sPeripheral,
    STD: I2sStandard,
    (STD, FMT): FrameFormat,
{
    /// Read samples while predicate returns `true`.
    ///
    /// The given closure must not block, otherwise communication problems may occur.
    pub fn read_while<F, T>(&mut self, mut predicate: F)
    where
        T: FromRawFrame<STD, FMT>,
        F: FnMut(T) -> bool,
    {
        loop {
            if self.sync {
                let status = self.driver.status();
                if status.rxne() {
                    if self.transfer_count >= self.frame.as_ref().len() as u8 {
                        self.transfer_count = 0;
                    }
                    self.frame.as_mut()[self.transfer_count as usize] =
                        self.driver.read_data_register();
                    self.transfer_count += 1;

                    // note: boolean operators are short-circuiting
                    if self.transfer_count >= self.frame.as_ref().len() as u8
                        && !predicate(T::from_raw(self.frame))
                    {
                        return;
                    }
                }
                if status.fre() || status.ovr() {
                    self.sync = false;
                    self.driver.read_data_register();
                    self.driver.status();
                    self.driver.disable();
                }
            } else if !self._ws_is_start() {
                self.transfer_count = 0;
                self.driver.enable();
                // ensure the ws line didn't change during sync process
                if !self._ws_is_start() {
                    self.sync = true;
                } else {
                    self.driver.disable();
                }
            }
        }
    }

    /// Read one audio frame and activate the I2s interface if disabled.
    ///
    /// To get the audio frame, this function need to be continuously called until the frame is
    /// returned
    pub fn read<T: FromRawFrame<STD, FMT>>(&mut self) -> nb::Result<T, Infallible> {
        if !self.sync {
            self.driver.disable();
            self.transfer_count = 0;
        }
        if self.sync {
            let status = self.driver.status();
            if status.rxne() {
                if self.transfer_count >= self.frame.as_ref().len() as u8 {
                    self.transfer_count = 0;
                }
                self.frame.as_mut()[self.transfer_count as usize] =
                    self.driver.read_data_register();
                self.transfer_count += 1;

                // note: boolean operators are short-circuiting
                if self.transfer_count >= self.frame.as_ref().len() as u8 {
                    return Ok(T::from_raw(self.frame));
                }
            }
            if status.fre() || status.ovr() {
                self.sync = false;
                //self.driver.read_data_register();
                //self.driver.status();
                self.driver.disable();
            }
        } else if !self._ws_is_start() {
            self.transfer_count = 0;
            self.driver.enable();
            self.driver.read_data_register();
            self.driver.status();
            // ensure the ws line didn't change during sync process
            if !self._ws_is_start() {
                self.sync = true;
            } else {
                self.driver.disable();
            }
        }
        Err(WouldBlock)
    }
}
