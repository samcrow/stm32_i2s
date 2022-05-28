//! Abstraction to transfer I2S data.
//!
//! API of this module give abstractions allowing to transfer I2S audio data while hiding the
//! hardware details. This module also have basis to implement the upcoming embedded-hale I2s
//! trait. The job is mainly done by [`I2sTransfer`], a type that wrap an I2sPeripheral to control
//! it.
//!
//! # Configure and instantiate transfer.
//!
//! [`I2sTransferConfig`] is used to create configuration of the i2s transfer:
//! ```no_run
//! let transfer_config = I2sTransferConfig::new_master()
//!     .receive()
//!     .standard(marker::Philips)
//!     .data_format(marker::Data24Channel32)
//!     .master_clock(true)
//!     .request_frequency(48_000);
//! ```
//! Then you can instantiate the transfer around an `I2sPeripheral`:
//! ```no_run
//! // instantiate from configuration
//! let transfer = transfer_config.i2s_driver(i2s_peripheral);
//!
//! // alternate way
//! let transfer = I2sTransfer::new(i2s_peripheral, transfer_config);
//! ```
use core::convert::Infallible;
use core::marker::PhantomData;

use nb::Error::WouldBlock;

use crate::driver::Channel::*;
use crate::driver::ClockPolarity;
use crate::driver::I2sDriver as Driver;
use crate::driver::I2sDriverConfig as DriverConfig;
use crate::I2sPeripheral;

pub use crate::marker::{self, *};

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
    driver_config: DriverConfig<MS, TR>,
    _std: PhantomData<STD>,
    _fmt: PhantomData<FMT>,
}

impl I2sTransferConfig<Slave, Transmit, Philips, Data16Channel16> {
    /// Create a new default slave configuration.
    pub fn new_slave() -> Self {
        Self {
            driver_config: DriverConfig::new_slave(),
            _std: PhantomData,
            _fmt: PhantomData,
        }
    }
}

impl I2sTransferConfig<Master, Transmit, Philips, Data16Channel16> {
    /// Create a new default master configuration.
    pub fn new_master() -> Self {
        Self {
            driver_config: DriverConfig::new_master(),
            _std: PhantomData,
            _fmt: PhantomData,
        }
    }
}

impl<MS, TR, STD, FMT> I2sTransferConfig<MS, TR, STD, FMT>
where
    FMT: DataFormat,
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
            frame_state: FrameState::LeftMsb,
            sync: false,
            _std: PhantomData,
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
            _std: PhantomData,
            _fmt: PhantomData,
        }
    }
    /// Configure transfer for receiving data.
    pub fn receive(self) -> I2sTransferConfig<MS, Receive, STD, FMT> {
        I2sTransferConfig::<MS, Receive, STD, FMT> {
            driver_config: self.driver_config.receive(),
            _std: PhantomData,
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
            driver_config: self.driver_config.standard(NEW_STD::VALUE),
            _std: PhantomData,
            _fmt: PhantomData,
        }
    }
    /// Select steady state clock polarity
    pub fn clock_polarity(self, polarity: ClockPolarity) -> Self {
        I2sTransferConfig::<MS, TR, STD, FMT> {
            driver_config: self.driver_config.clock_polarity(polarity),
            _std: PhantomData,
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
            _std: PhantomData,
            _fmt: PhantomData,
        }
    }

    /// Convert to a slave configuration. This delete Master Only Settings.
    pub fn to_slave(self) -> I2sTransferConfig<Slave, TR, STD, FMT> {
        I2sTransferConfig::<Slave, TR, STD, FMT> {
            driver_config: self.driver_config.to_slave(),
            _std: PhantomData,
            _fmt: PhantomData,
        }
    }

    /// Convert to a master configuration.
    pub fn to_master(self) -> I2sTransferConfig<Master, TR, STD, FMT> {
        I2sTransferConfig::<Master, TR, STD, FMT> {
            driver_config: self.driver_config.to_master(),
            _std: PhantomData,
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
            _std: PhantomData,
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
            _std: PhantomData,
            _fmt: PhantomData,
        }
    }

    /// Request an audio sampling frequency. The effective audio sampling frequency may differ.
    pub fn request_frequency(self, freq: u32) -> Self {
        I2sTransferConfig::<Master, TR, STD, FMT> {
            driver_config: self.driver_config.request_frequency(freq),
            _std: PhantomData,
            _fmt: PhantomData,
        }
    }

    /// Require exactly this audio sampling frequency.
    ///
    /// If the required frequency can not bet set, Instantiate a transfer will panics.
    pub fn require_frequency(self, freq: u32) -> Self {
        I2sTransferConfig::<Master, TR, STD, FMT> {
            driver_config: self.driver_config.require_frequency(freq),
            _std: PhantomData,
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
    FMT: DataFormat,
{
    driver: Driver<I, Mode<MS, TR>>,
    frame: FMT::AudioFrame,
    frame_state: FrameState,
    sync: bool,
    _std: PhantomData<STD>,
    _fmt: PhantomData<FMT>,
}

impl<I, MS, TR, STD, FMT> I2sTransfer<I, MS, TR, STD, FMT>
where
    I: I2sPeripheral,
    STD: I2sStandard,
    FMT: DataFormat,
{
    /// When `true` the level on WS line make start a slave. The slave must be enabled before this
    /// level is set.
    #[inline]
    fn _ws_is_start(&self) -> bool {
        match STD::WS_START_LEVEL {
            false => self.driver.ws_is_low(),
            true => self.driver.ws_is_high(),
        }
    }
}

/// Constructors and Destructors
impl<I, MS, TR, STD, FMT> I2sTransfer<I, MS, TR, STD, FMT>
where
    I: I2sPeripheral,
    FMT: DataFormat,
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
    FMT: DataFormat,
{
    /// Activate the I2s interface.
    pub fn begin(&mut self) {
        self.driver.enable()
    }

    /// Deactivate the I2s interface and reset internal state
    pub fn end(&mut self) {
        self.driver.disable();
        self.frame = Default::default();
        self.frame_state = FrameState::LeftMsb;
        self.sync = false;
    }
}

impl<I, TR, STD, FMT> I2sTransfer<I, Master, TR, STD, FMT>
where
    I: I2sPeripheral,
    FMT: DataFormat,
{
    pub fn sample_rate(&self) -> u32 {
        self.driver.sample_rate()
    }
}

impl<I, STD, FMT> I2sTransfer<I, Master, Transmit, STD, FMT>
where
    I: I2sPeripheral,
    FMT: Data16 + DataFormat<AudioFrame = (i16, i16)>,
{
    pub fn write_iter<ITER>(&mut self, samples: ITER)
    where
        ITER: IntoIterator<Item = (i16, i16)>,
    {
        let mut samples = samples.into_iter();
        self.driver.enable();
        loop {
            let status = self.driver.status();
            if status.txe() {
                let data;
                match self.frame_state {
                    LeftMsb => {
                        let smpl = samples.next();
                        //breaking here ensure the last frame is fully transmitted
                        if smpl.is_none() {
                            break;
                        }
                        self.frame = smpl.unwrap();
                        data = (self.frame.0) as u16;
                        self.frame_state = RightMsb;
                    }
                    RightMsb => {
                        data = (self.frame.1) as u16;
                        self.frame_state = LeftMsb;
                    }
                    _ => unreachable!(),
                }
                self.driver.write_data_register(data);
            }
        }
    }

    /// Write one audio frame. Activate the I2s interface if disabled.
    ///
    /// To fully transmit the frame, this function need to be continuously called until next
    /// frame can be written.
    pub fn write(&mut self, frame: (i16, i16)) -> nb::Result<(), Infallible> {
        self.driver.enable();
        let status = self.driver.status();
        if status.txe() {
            match self.frame_state {
                LeftMsb => {
                    self.frame = frame;
                    let data = (self.frame.0) as u16;
                    self.driver.write_data_register(data);
                    self.frame_state = RightMsb;
                    return Ok(());
                }
                RightMsb => {
                    let data = (self.frame.1) as u16;
                    self.driver.write_data_register(data);
                    self.frame_state = LeftMsb;
                }
                _ => unreachable!(),
            }
        }
        Err(WouldBlock)
    }
}

impl<I, STD> I2sTransfer<I, Master, Transmit, STD, Data32Channel32>
where
    I: I2sPeripheral,
{
    pub fn write_iter<ITER>(&mut self, samples: ITER)
    where
        ITER: IntoIterator<Item = (i32, i32)>,
    {
        let mut samples = samples.into_iter();
        self.driver.enable();
        loop {
            let status = self.driver.status();
            if status.txe() {
                let data;
                match self.frame_state {
                    LeftMsb => {
                        let smpl = samples.next();
                        //breaking here ensure the last frame is fully transmitted
                        if smpl.is_none() {
                            break;
                        }
                        self.frame = smpl.unwrap();
                        data = (self.frame.0 as u32 >> 16) as u16;
                        self.frame_state = LeftLsb;
                    }
                    LeftLsb => {
                        data = (self.frame.0 as u32 & 0xFFFF) as u16;
                        self.frame_state = RightMsb;
                    }
                    RightMsb => {
                        data = (self.frame.1 as u32 >> 16) as u16;
                        self.frame_state = RightLsb;
                    }
                    RightLsb => {
                        data = (self.frame.1 as u32 & 0xFFFF) as u16;
                        self.frame_state = LeftMsb;
                    }
                }
                self.driver.write_data_register(data);
            }
        }
    }

    /// Write one audio frame. Activate the I2s interface if disabled.
    ///
    /// To fully transmit the frame, this function need to be continuously called until next
    /// frame can be written.
    pub fn write(&mut self, frame: (i32, i32)) -> nb::Result<(), Infallible> {
        self.driver.enable();
        let status = self.driver.status();
        if status.txe() {
            match self.frame_state {
                LeftMsb => {
                    self.frame = frame;
                    let data = (self.frame.0 as u32 >> 16) as u16;
                    self.driver.write_data_register(data);
                    self.frame_state = LeftLsb;
                    return Ok(());
                }
                LeftLsb => {
                    let data = (self.frame.0 as u32 & 0xFFFF) as u16;
                    self.driver.write_data_register(data);
                    self.frame_state = RightMsb;
                }
                RightMsb => {
                    let data = (self.frame.1 as u32 >> 16) as u16;
                    self.driver.write_data_register(data);
                    self.frame_state = RightLsb;
                }
                RightLsb => {
                    let data = (frame.1 as u32 & 0xFFFF) as u16;
                    self.driver.write_data_register(data);
                    self.frame_state = LeftMsb;
                }
            }
        }
        Err(WouldBlock)
    }
}

impl<I, STD, FMT> I2sTransfer<I, Slave, Transmit, STD, FMT>
where
    I: I2sPeripheral,
    STD: I2sStandard,
    FMT: Data16 + DataFormat<AudioFrame = (i16, i16)>,
{
    //TODO WS_line sensing is protocol dependent
    pub fn write_iter<ITER>(&mut self, samples: ITER)
    where
        ITER: IntoIterator<Item = (i16, i16)>,
    {
        let mut samples = samples.into_iter();
        loop {
            if self.sync {
                let status = self.driver.status();
                if status.txe() {
                    let data;
                    match self.frame_state {
                        LeftMsb => {
                            let smpl = samples.next();
                            //breaking here ensure the last frame is fully transmitted
                            if smpl.is_none() {
                                break;
                            }
                            self.frame = smpl.unwrap();
                            data = self.frame.0 as u16;
                            self.frame_state = RightMsb;
                        }
                        RightMsb => {
                            data = self.frame.1 as u16;
                            self.frame_state = LeftMsb;
                        }
                        _ => unreachable!(),
                    }
                    self.driver.write_data_register(data);
                }
                if status.fre() || status.udr() {
                    self.sync = false;
                    self.driver.disable();
                }
            } else if !self._ws_is_start() {
                // data register may (or not) already contain data, causing uncertainty about next
                // time txe flag is set. Writing it remove the uncertainty.
                let smpl = samples.next();
                //breaking here ensure the last frame is fully transmitted
                if smpl.is_none() {
                    break;
                }
                self.frame = smpl.unwrap();
                let data = self.frame.0 as u16;
                self.driver.write_data_register(data);
                self.frame_state = RightMsb;
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
    /// Write one audio frame. Activate the I2s interface if disabled.
    ///
    /// To fully transmit the frame, this function need to be continuously called until next
    /// frame can be written.
    pub fn write(&mut self, frame: (i16, i16)) -> nb::Result<(), Infallible> {
        if self.sync {
            let status = self.driver.status();
            if status.txe() {
                match self.frame_state {
                    LeftMsb => {
                        self.frame = frame;
                        let data = self.frame.0 as u16;
                        self.driver.write_data_register(data);
                        self.frame_state = RightMsb;
                        return Ok(());
                    }
                    RightMsb => {
                        let data = self.frame.1 as u16;
                        self.driver.write_data_register(data);
                        self.frame_state = LeftMsb;
                    }
                    _ => unreachable!(),
                }
            }
            if status.fre() || status.udr() {
                self.sync = false;
                self.driver.disable();
            }
        } else if !self._ws_is_start() {
            // data register may (or not) already contain data, causing uncertainty about next
            // time txe flag is set. Writing it remove the uncertainty.
            let data = self.frame.0 as u16;
            self.driver.write_data_register(data);
            self.frame_state = RightMsb;
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

impl<I, STD> I2sTransfer<I, Slave, Transmit, STD, Data32Channel32>
where
    I: I2sPeripheral,
    STD: I2sStandard,
{
    #[inline]
    // Can't make it work now
    pub fn write_iter<ITER>(&mut self, samples: ITER)
    where
        ITER: IntoIterator<Item = (i32, i32)>,
    {
        let mut samples = samples.into_iter();
        loop {
            if self.sync {
                let status = self.driver.status();
                if status.txe() {
                    let data;
                    match self.frame_state {
                        LeftMsb => {
                            let smpl = samples.next();
                            //breaking here ensure the last frame is fully transmitted
                            if smpl.is_none() {
                                break;
                            }
                            self.frame = smpl.unwrap();
                            data = (self.frame.0 as u32 >> 16) as u16;
                            self.frame_state = LeftLsb;
                        }
                        LeftLsb => {
                            data = (self.frame.0 as u32 & 0xFFFF) as u16;
                            self.frame_state = RightMsb;
                        }
                        RightMsb => {
                            data = (self.frame.1 as u32 >> 16) as u16;
                            self.frame_state = RightLsb;
                        }
                        RightLsb => {
                            data = (self.frame.1 as u32 & 0xFFFF) as u16;
                            self.frame_state = LeftMsb;
                        }
                    }
                    self.driver.write_data_register(data);
                }
                if status.fre() || status.udr() {
                    self.sync = false;
                    self.driver.disable();
                }
            } else if !self._ws_is_start() {
                // data register may (or not) already contain data, causing uncertainty about next
                // time txe flag is set. Writing it remove the uncertainty.
                let smpl = samples.next();
                //breaking here ensure the last frame is fully transmitted
                if smpl.is_none() {
                    break;
                }
                self.frame = smpl.unwrap();
                let data = (self.frame.0 as u32 >> 16) as u16;
                self.driver.write_data_register(data);
                self.frame_state = LeftLsb;
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

    /// Write one audio frame. Activate the I2s interface if disabled.
    ///
    /// To fully transmit the frame, this function need to be continuously called until next
    /// frame can be written.
    pub fn write(&mut self, frame: (i32, i32)) -> nb::Result<(), Infallible> {
        if self.sync {
            let status = self.driver.status();
            if status.txe() {
                match self.frame_state {
                    LeftMsb => {
                        self.frame = frame;
                        let data = (self.frame.0 as u32 >> 16) as u16;
                        self.driver.write_data_register(data);
                        self.frame_state = LeftLsb;
                        return Ok(());
                    }
                    LeftLsb => {
                        let data = (self.frame.0 as u32 & 0xFFFF) as u16;
                        self.driver.write_data_register(data);
                        self.frame_state = RightMsb;
                    }
                    RightMsb => {
                        let data = (self.frame.1 as u32 >> 16) as u16;
                        self.driver.write_data_register(data);
                        self.frame_state = RightLsb;
                    }
                    RightLsb => {
                        let data = (frame.1 as u32 & 0xFFFF) as u16;
                        self.driver.write_data_register(data);
                        self.frame_state = LeftMsb;
                    }
                }
            }
            if status.fre() || status.udr() {
                self.sync = false;
                self.driver.disable();
            }
        } else if !self._ws_is_start() {
            // data register may (or not) already contain data, causing uncertainty about next
            // time txe flag is set. Writing it remove the uncertainty.
            let data = (self.frame.0 as u32 >> 16) as u16;
            self.driver.write_data_register(data);
            self.frame_state = LeftLsb;
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

impl<I, STD, FMT> I2sTransfer<I, Master, Receive, STD, FMT>
where
    I: I2sPeripheral,
    FMT: Data16 + DataFormat<AudioFrame = (i16, i16)>,
    STD: ChannelFlag,
{
    /// Read samples while predicate return `true`.
    ///
    /// The given closure must not block, otherwise communication problems may occur.
    pub fn read_while<F>(&mut self, mut predicate: F)
    where
        F: FnMut((i16, i16)) -> bool,
    {
        self.driver.enable();
        loop {
            let status = self.driver.status();
            if status.rxne() {
                match status.chside() {
                    Left => {
                        let data = self.driver.read_data_register();
                        self.frame.0 = data as i16;
                        self.frame_state = RightMsb;
                    }
                    Right => {
                        let data = self.driver.read_data_register();
                        self.frame.1 = data as i16;
                        self.frame_state = LeftMsb;
                        if !predicate(self.frame) {
                            return;
                        }
                    }
                }
            }
            if status.ovr() {
                self.driver.read_data_register();
                self.driver.status();
            }
        }
    }

    /// Read one audio frame. Activate the I2s interface if disabled.
    ///
    /// To get the audio frame, this function need to be continuously called until the frame is
    /// returned
    pub fn read(&mut self) -> nb::Result<(i16, i16), Infallible> {
        self.driver.enable();
        let status = self.driver.status();
        if status.rxne() {
            match status.chside() {
                Left => {
                    let data = self.driver.read_data_register();
                    self.frame.0 = data as i16;
                    self.frame_state = RightMsb;
                }
                Right => {
                    let data = self.driver.read_data_register();
                    self.frame.1 = data as i16;
                    self.frame_state = LeftMsb;
                    return Ok(self.frame);
                }
            }
        }
        if status.ovr() {
            self.driver.read_data_register();
            self.driver.status();
        }
        Err(WouldBlock)
    }
}

impl<I, STD> I2sTransfer<I, Master, Receive, STD, Data32Channel32>
where
    I: I2sPeripheral,
    STD: ChannelFlag,
{
    /// Read samples while predicate return `true`.
    ///
    /// The given closure must not block, otherwise communication problems may occur.
    pub fn read_while<F>(&mut self, mut predicate: F)
    where
        F: FnMut((i32, i32)) -> bool,
    {
        self.driver.enable();
        loop {
            let status = self.driver.status();
            if status.rxne() {
                let data = self.driver.read_data_register();
                match (self.frame_state, status.chside()) {
                    (LeftMsb, Left) => {
                        self.frame.0 = (data as i32) << 16;
                        self.frame_state = LeftLsb;
                    }
                    (LeftLsb, Left) => {
                        self.frame.0 |= data as i32;
                        self.frame_state = RightMsb;
                    }
                    (RightMsb, Right) => {
                        self.frame.1 = (data as i32) << 16;
                        self.frame_state = RightLsb;
                    }
                    (RightLsb, Right) => {
                        self.frame.1 |= data as i32;
                        self.frame_state = LeftMsb;
                        if !predicate(self.frame) {
                            return;
                        }
                    }
                    // in case of ovr this resynchronize at start of new frame
                    _ => self.frame_state = LeftMsb,
                }
            }
            if status.ovr() {
                self.driver.read_data_register();
                self.driver.status();
                self.frame_state = LeftMsb;
            }
        }
    }

    /// Read one audio frame. Activate the I2s interface if disabled.
    ///
    /// To get the audio frame, this function need to be continuously called until the frame is
    /// returned
    pub fn read(&mut self) -> nb::Result<(i32, i32), Infallible> {
        self.driver.enable();
        let status = self.driver.status();
        if status.rxne() {
            let data = self.driver.read_data_register();
            match (self.frame_state, status.chside()) {
                (LeftMsb, Left) => {
                    self.frame.0 = (data as i32) << 16;
                    self.frame_state = LeftLsb;
                }
                (LeftLsb, Left) => {
                    self.frame.0 |= data as i32;
                    self.frame_state = RightMsb;
                }
                (RightMsb, Right) => {
                    self.frame.1 = (data as i32) << 16;
                    self.frame_state = RightLsb;
                }
                (RightLsb, Right) => {
                    self.frame.1 |= data as i32;
                    self.frame_state = LeftMsb;
                    return Ok(self.frame);
                }
                // in case of ovr this resynchronize at start of new frame
                _ => self.frame_state = LeftMsb,
            }
            if status.ovr() {
                self.driver.read_data_register();
                self.driver.status();
                self.frame_state = LeftMsb;
            }
        }
        Err(WouldBlock)
    }
}

impl<I, STD, FMT> I2sTransfer<I, Slave, Receive, STD, FMT>
where
    I: I2sPeripheral,
    FMT: Data16 + DataFormat<AudioFrame = (i16, i16)>,
    STD: ChannelFlag,
{
    /// Read samples while predicate return `true`.
    ///
    /// The given closure must not block, otherwise communication problems may occur.
    pub fn read_while<F>(&mut self, mut predicate: F)
    where
        F: FnMut((i16, i16)) -> bool,
    {
        loop {
            if self.sync {
                let status = self.driver.status();
                if status.rxne() {
                    let data = self.driver.read_data_register();
                    match self.frame_state {
                        LeftMsb => {
                            self.frame.0 = data as i16;
                            self.frame_state = RightMsb;
                        }
                        RightMsb => {
                            self.frame.1 = data as i16;
                            self.frame_state = LeftMsb;
                            if !predicate(self.frame) {
                                return;
                            }
                        }
                        _ => unreachable!(),
                    }
                }
                if status.fre() || status.ovr() {
                    self.sync = false;
                    self.driver.read_data_register();
                    self.driver.status();
                    self.driver.disable();
                }
            } else if !self._ws_is_start() {
                self.frame_state = LeftMsb;
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

    /// Read one audio frame. Activate the I2s interface if disabled.
    ///
    /// To get the audio frame, this function need to be continuously called until the frame is
    /// returned
    pub fn read(&mut self) -> nb::Result<(i16, i16), Infallible> {
        if !self.sync {
            self.driver.disable();
            self.frame_state = RightMsb;
        }
        if self.sync {
            let status = self.driver.status();
            if status.rxne() {
                let data = self.driver.read_data_register();
                match self.frame_state {
                    LeftMsb => {
                        self.frame.0 = data as i16;
                        self.frame_state = RightMsb;
                    }
                    RightMsb => {
                        self.frame.1 = data as i16;
                        self.frame_state = LeftMsb;
                        return Ok(self.frame);
                    }
                    _ => unreachable!(),
                }
            }
            if status.fre() || status.ovr() {
                self.sync = false;
                //self.driver.read_data_register();
                //self.driver.status();
                self.driver.disable();
            }
        } else if !self._ws_is_start() {
            //defmt::println!("sycing");
            self.frame_state = RightMsb;
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

impl<I, STD> I2sTransfer<I, Slave, Receive, STD, Data32Channel32>
where
    I: I2sPeripheral,
    STD: ChannelFlag,
{
    /// Read samples while predicate return `true`.
    ///
    /// The given closure must not block, otherwise communication problems may occur.
    pub fn read_while<F>(&mut self, mut predicate: F)
    where
        F: FnMut((i32, i32)) -> bool,
    {
        loop {
            if self.sync {
                let status = self.driver.status();
                if status.rxne() {
                    let data = self.driver.read_data_register();
                    match self.frame_state {
                        LeftMsb => {
                            self.frame.0 = (data as i32) << 16;
                            self.frame_state = LeftLsb;
                        }
                        LeftLsb => {
                            self.frame.0 |= data as i32;
                            self.frame_state = RightMsb;
                        }
                        RightMsb => {
                            self.frame.1 = (data as i32) << 16;
                            self.frame_state = RightLsb;
                        }
                        RightLsb => {
                            self.frame.1 |= data as i32;
                            self.frame_state = LeftMsb;
                            if !predicate(self.frame) {
                                return;
                            }
                        }
                    }
                }
                if status.fre() || status.ovr() {
                    self.sync = false;
                    self.driver.read_data_register();
                    self.driver.status();
                    self.driver.disable();
                }
            } else if !self._ws_is_start() {
                self.frame_state = LeftMsb;
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

    /// Read one audio frame. Activate the I2s interface if disabled.
    ///
    /// To get the audio frame, this function need to be continuously called until the frame is
    /// returned
    pub fn read(&mut self) -> nb::Result<(i32, i32), Infallible> {
        if !self.sync {
            self.driver.disable();
            self.frame_state = RightMsb;
        }
        if self.sync {
            let status = self.driver.status();
            if status.rxne() {
                let data = self.driver.read_data_register();
                match self.frame_state {
                    LeftMsb => {
                        self.frame.0 = (data as i32) << 16;
                        self.frame_state = LeftLsb;
                    }
                    LeftLsb => {
                        self.frame.0 |= data as i32;
                        self.frame_state = RightMsb;
                    }
                    RightMsb => {
                        self.frame.1 = (data as i32) << 16;
                        self.frame_state = RightLsb;
                    }
                    RightLsb => {
                        self.frame.1 |= data as i32;
                        self.frame_state = LeftMsb;
                        return Ok(self.frame);
                    }
                }
            }
            if status.fre() || status.ovr() {
                self.sync = false;
                //self.driver.read_data_register();
                //self.driver.status();
                self.driver.disable();
            }
        } else if !self._ws_is_start() {
            //defmt::println!("sycing");
            self.frame_state = RightMsb;
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
