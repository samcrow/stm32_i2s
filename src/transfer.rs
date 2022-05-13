//! Abstraction for I2S transfer
//!
//!
use core::convert::Infallible;
use nb::Error::WouldBlock;

use crate::Config as DriverConfig;
use crate::I2sDriver as Driver;
use crate::*;

#[derive(Debug, Clone, Copy)]
/// I2s TransferConfiguration builder.
///
///  - `MS`: `Master` or `Slave`
///  - `TR`: `Transmit` or `Receive`
///  - `FMT`: Frame Format marker, eg `Data16Channel16`
pub struct TransferConfig<MS, TR, FMT> {
    driver_config: DriverConfig<MS, TR>,
    _fmt: PhantomData<FMT>,
}

impl TransferConfig<Slave, Transmit, Data16Channel16> {
    /// Create a new default slave configuration.
    pub fn new_slave() -> Self {
        Self {
            driver_config: DriverConfig::new_slave(),
            _fmt: PhantomData,
        }
    }
}

impl TransferConfig<Master, Transmit, Data16Channel16> {
    /// Create a new default master configuration.
    pub fn new_master() -> Self {
        Self {
            driver_config: DriverConfig::new_master(),
            _fmt: PhantomData,
        }
    }
}

impl<MS, TR, FMT> TransferConfig<MS, TR, FMT>
where
    FMT: DataFormat,
{
    /// Create a `Transfer` object.
    pub fn i2s_transfer<I: I2sPeripheral>(self, i2s_peripheral: I) -> Transfer<I, MS, TR, FMT> {
        let driver = self.driver_config.i2s_driver(i2s_peripheral);
        Transfer::<I, MS, TR, FMT> {
            driver,
            frame: Default::default(),
            frame_state: FrameState::LeftMsb,
            _fmt: PhantomData,
        }
    }
}

impl Default for TransferConfig<Slave, Transmit, Data16Channel16> {
    /// Create a default configuration. It correspond to a default slave configuration.
    fn default() -> Self {
        Self::new_slave()
    }
}

impl<MS, TR, FMT> TransferConfig<MS, TR, FMT> {
    /// Configure transfert for transmission.
    pub fn transmit(self) -> TransferConfig<MS, Transmit, FMT> {
        TransferConfig::<MS, Transmit, FMT> {
            driver_config: self.driver_config.transmit(),
            _fmt: PhantomData,
        }
    }
    /// TransferConfigure in transmit mode
    pub fn receive(self) -> TransferConfig<MS, Receive, FMT> {
        TransferConfig::<MS, Receive, FMT> {
            driver_config: self.driver_config.receive(),
            _fmt: PhantomData,
        }
    }
    /// Select the I2s standard to use
    pub fn standard(self, standard: I2sStandard) -> Self {
        TransferConfig::<MS, TR, FMT> {
            driver_config: self.driver_config.standard(standard),
            _fmt: PhantomData,
        }
    }
    /// Select steady state clock polarity
    pub fn clock_polarity(self, polarity: ClockPolarity) -> Self {
        TransferConfig::<MS, TR, FMT> {
            driver_config: self.driver_config.clock_polarity(polarity),
            _fmt: PhantomData,
        }
    }

    /// Select data format
    #[allow(non_camel_case_types)]
    pub fn data_format<NEW_FMT>(self, _format: NEW_FMT) -> TransferConfig<MS, TR, NEW_FMT>
    where
        NEW_FMT: marker::DataFormat,
    {
        TransferConfig::<MS, TR, NEW_FMT> {
            driver_config: self.driver_config.data_format(NEW_FMT::VALUE),
            _fmt: PhantomData,
        }
    }

    /// Convert to a slave configuration. This delete Master Only Settings.
    pub fn to_slave(self) -> TransferConfig<Slave, TR, FMT> {
        TransferConfig::<Slave, TR, FMT> {
            driver_config: self.driver_config.to_slave(),
            _fmt: PhantomData,
        }
    }

    /// Convert to a master configuration.
    pub fn to_master(self) -> TransferConfig<Master, TR, FMT> {
        TransferConfig::<Master, TR, FMT> {
            driver_config: self.driver_config.to_master(),
            _fmt: PhantomData,
        }
    }
}

impl<TR, FMT> TransferConfig<Master, TR, FMT> {
    /// Enable/Disable Master Clock. Affect the effective sampling rate.
    ///
    /// This can be only set and only have meaning for Master mode.
    pub fn master_clock(self, enable: bool) -> Self {
        TransferConfig::<Master, TR, FMT> {
            driver_config: self.driver_config.master_clock(enable),
            _fmt: PhantomData,
        }
    }

    /// TransferConfigure audio frequency by setting the prescaler with an odd factor and a divider.
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
        TransferConfig::<Master, TR, FMT> {
            driver_config: self.driver_config.prescaler(odd, div),
            _fmt: PhantomData,
        }
    }

    /// Request an audio sampling frequency. The effective audio sampling frequency may differ.
    pub fn request_frequency(self, freq: u32) -> Self {
        TransferConfig::<Master, TR, FMT> {
            driver_config: self.driver_config.request_frequency(freq),
            _fmt: PhantomData,
        }
    }

    /// Require exactly this audio sampling frequency.
    ///
    /// If the required frequency can not bet set, Instantiate the driver will produce a error
    pub fn require_frequency(self, freq: u32) -> Self {
        TransferConfig::<Master, TR, FMT> {
            driver_config: self.driver_config.require_frequency(freq),
            _fmt: PhantomData,
        }
    }
}

/// Part of the frame we currently transmitting or receiving
#[derive(Debug)]
enum FrameState {
    LeftMsb,
    LeftLsb,
    RightMsb,
    RightLsb,
}
use FrameState::*;

pub struct Transfer<I, MS, TR, FMT>
where
    I: I2sPeripheral,
    FMT: DataFormat,
{
    driver: Driver<I, Mode<MS, TR>>,
    frame: FMT::AudioFrame,
    frame_state: FrameState,
    _fmt: PhantomData<FMT>,
}

/// Constructors and Destructors
impl<I, MS, TR, FMT> Transfer<I, MS, TR, FMT>
where
    I: I2sPeripheral,
    FMT: DataFormat,
{
    /// Instantiate and configure an i2s driver.
    pub fn new(i2s_peripheral: I, config: TransferConfig<MS, TR, FMT>) -> Self {
        config.i2s_transfer(i2s_peripheral)
    }

    /// Destroy the transfer, release the owned i2s device and reset it's configuration.
    pub fn release(self) -> I {
        self.driver.release()
    }
}

impl<I, MS, TR, FMT> Transfer<I, MS, TR, FMT>
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
    }
}

impl<I, TR, FMT> Transfer<I, Master, TR, FMT>
where
    I: I2sPeripheral,
    FMT: DataFormat,
{
    pub fn sample_rate(&self) -> u32 {
        self.driver.sample_rate()
    }
}

impl<I, FMT> Transfer<I, Master, Transmit, FMT>
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

impl<I> Transfer<I, Master, Transmit, Data32Channel32>
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

impl<I> Transfer<I, Slave, Transmit, Data32Channel32>
where
    I: I2sPeripheral,
{
    #[inline]
    // Can't make it work now
    pub fn write_iter<ITER>(&mut self, samples: ITER)
    where
        ITER: IntoIterator<Item = (i32, i32)>,
    {
        let mut frame_state = LeftMsb;
        let mut frame = (0, 0);
        let mut samples = samples.into_iter();
        self.driver.disable();
        self.driver.status();
        // initial synchronisation
        while !self.driver.ws_is_high() {}
        self.driver.enable();
        loop {
            let status = self.driver.status();
            if status.txe() {
                let data;
                match (frame_state, status.chside()) {
                    (LeftMsb, Channel::Left) => {
                        let smpl = samples.next();
                        //breaking here ensure the last frame is fully transmitted
                        if smpl.is_none() {
                            break;
                        }
                        frame = smpl.unwrap();
                        data = (frame.0 as u32 >> 16) as u16;
                        frame_state = LeftLsb;
                    }
                    (LeftLsb, _) => {
                        data = (frame.0 as u32 & 0xFFFF) as u16;
                        frame_state = RightMsb;
                    }
                    (RightMsb, _) => {
                        data = (frame.1 as u32 >> 16) as u16;
                        frame_state = RightLsb;
                    }
                    (RightLsb, _) => {
                        data = (frame.1 as u32 & 0xFFFF) as u16;
                        frame_state = LeftMsb;
                    }
                    _ => {
                        data = 0;
                        frame_state = LeftMsb;
                    }
                }
                self.driver.write_data_register(data);
            }
            if status.fre() {
                rtt_target::rprintln!("{} {}", status.fre(), status.udr());
                self.driver.disable();
                frame_state = LeftMsb;
                while !self.driver.ws_is_high() {}
                self.driver.enable();
            }
            if status.udr() {
                rtt_target::rprintln!("udr");
            }
        }
        self.driver.disable();
    }
}
