//! Types definitions for I2S driver.
//!
//! API of this module provides thin abstractions  that try to give access to relevant hardware
//! details while preventing irrelevant or meaningless operation. This allow precise and concise
//! control of a SPI/I2S peripheral. It's meant for advanced usage, for example with interrupt or
//! DMA. The job is mainly done by [`I2sDriver`], a type that wrap an [`I2sPeripheral`] to control
//! it.
//!
//! # Configure and instantiate driver.
//!
//! [`I2sDriverConfig`] is used to create configuration of the i2s driver:
//! ```no_run
//! # use stm32_i2s_v12x::driver::*;
//! let driver_config = I2sDriverConfig::new_master()
//!     .receive()
//!     .standard(Philips)
//!     .data_format(DataFormat::Data16Channel32)
//!     .master_clock(true)
//!     .request_frequency(48_000);
//! ```
//! Then you can instantiate the driver around an `I2sPeripheral`:
//! ```ignore
//! // instantiate from configuration
//! let driver = driver_config.i2s_driver(i2s_peripheral);
//!
//! // alternate way
//! let driver = I2sDriver::new(i2s_peripheral, driver_config);
//! ```
//!
//! # Usage
//!
//! `I2sDriver` actually give direct access to hardware, there isn't concept of audio data with it,
//! it's up to the user to reconstruct this information by controlling the hardware and using
//! available informations.
//!
//! Pseudocode example when driver is configured to receive 16 bit audio data:
//! ```ignore
//! let status = driver.status();
//! if status.rxne() {
//!     let data = driver.read_data_register();
//!     match status.chside() {
//!         Channel::Left => /* `data` contains left channel audio data */,
//!         Channel::Right => /* `data` contains right channel audio data */,
//!     }
//! }
//! ```
use core::marker::PhantomData;

use crate::pac::spi1::RegisterBlock;
use crate::pac::spi1::{i2spr, sr};
use crate::{I2sPeripheral, WsPin};

pub use crate::marker::{self, *};

/// The channel associated with a sample
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Channel {
    /// Left channel
    Left,
    /// Right channel
    Right,
}

/// Content of the status register.
///
///  - `MS`: `Master` or `Slave`
///  - `TR`: `Transmit` or `Receive`
///  - `STD`: I2S standard, eg `Philips`
pub struct Status<MS, TR, STD> {
    value: sr::R,
    _ms: PhantomData<MS>,
    _tr: PhantomData<TR>,
    _std: PhantomData<STD>,
}

impl<MS, TR, STD> Status<MS, TR, STD> {
    /// Get the BSY flag. If `true` the I2s device is busy communicating.
    pub fn bsy(&self) -> bool {
        self.value.bsy().bit()
    }
}

impl<MS, TR, STD> Status<MS, TR, STD>
where
    STD: marker::ChannelFlag,
{
    /// Get the CHSIDE flag. It indicate the channel has been received or to be transmitted.
    ///
    /// This flag is updated when TXE or RXNE flags are set. This flag is meaningless and therefore
    /// not reliable is case of error. This flag is not available in PCM standard because it's also
    /// meaningless in this case.
    pub fn chside(&self) -> Channel {
        match self.value.chside().bit() {
            false => Channel::Left,
            true => Channel::Right,
        }
    }
}

impl<TR, STD> Status<Slave, TR, STD> {
    /// Get the FRE flag. If `true` a frame error occurred.
    ///
    /// This flag is set by hardware when the WS line change at an unexpected moment. Usually, this
    /// indicate a synchronisation issue. This flag can only be set in Slave mode and therefore can
    /// only be read in this mode.
    ///
    /// This flag is cleared when reading the status register.
    pub fn fre(&self) -> bool {
        self.value.fre().bit()
    }
}

impl<MS, STD> Status<MS, Receive, STD> {
    /// Get the OVR flag. If `true` an overrun error occurred.
    ///
    /// This flag is set when data are received and the previous data have not yet been read. As a
    /// result, the incoming data are lost. Since this flag can happen only in Receive mode, it can
    /// only be read in this mode.
    ///
    /// This flag is cleared by a read operation on the data register followed by a read to the
    /// status register.
    pub fn ovr(&self) -> bool {
        self.value.ovr().bit()
    }

    /// Get the RXNE flag. If `true` a valid received data is present in the Rx buffer.
    ///
    /// This flag can only happen in reception mode and therefore can only be read in this mode.
    ///
    /// This flag is cleared when the data register is read.
    pub fn rxne(&self) -> bool {
        self.value.rxne().bit()
    }
}

impl<MS, STD> Status<MS, Transmit, STD> {
    /// Get the TXE flag. If `true` the Tx buffer is empty and the next data can be loaded into it.
    ///
    /// This flag can only happen in transmission mode and therefore can only be read in this mode.
    ///
    /// This flag is cleared by writing into the data register or by disabling the I2s peripheral.
    pub fn txe(&self) -> bool {
        self.value.txe().bit()
    }
}

impl<STD> Status<Slave, Transmit, STD> {
    /// Get the UDR flag. If `true` an underrun error occurred.
    ///
    /// This flag is set when the first clock for data transmission appears while the software has
    /// not yet loaded any value into the data register. This flag can only be set in Slave
    /// Transmit mode and therefore can only be read in this mode.
    ///
    /// This flag is cleared by reading the status register.
    pub fn udr(&self) -> bool {
        self.value.udr().bit()
    }
}

#[derive(Debug, Clone, Copy)]
enum SlaveOrMaster {
    Slave,
    Master,
}

#[derive(Debug, Clone, Copy)]
enum TransmitOrReceive {
    Transmit,
    Receive,
}

/// Various ways to specify sampling frequency.
#[derive(Debug, Clone, Copy)]
enum Frequency {
    Prescaler(bool, u8),
    Request(u32),
    Require(u32),
}

/// Those thing are not part of the public API but appear on public trait.
pub(crate) mod private {
    #[derive(Debug, Clone, Copy)]
    /// I2s standard selection.
    pub enum I2sStandard {
        /// Philips I2S
        Philips,
        /// MSB Justified
        Msb,
        /// LSB Justified
        Lsb,
        /// PCM with short frame synchronisation.
        PcmShortSync,
        /// PCM with long frame synchronisation.
        PcmLongSync,
    }
}
pub(crate) use private::I2sStandard;

/// Steady state clock polarity
#[derive(Debug, Clone, Copy)]
pub enum ClockPolarity {
    /// Clock low when idle
    IdleLow,
    /// Clock high when idle
    IdleHigh,
}

/// Data length to be transferred and channel length
#[derive(Debug, Clone, Copy)]
pub enum DataFormat {
    /// 16 bit data length on 16 bit wide channel
    Data16Channel16,
    /// 16 bit data length on 32 bit wide channel
    Data16Channel32,
    /// 24 bit data length on 32 bit wide channel
    Data24Channel32,
    /// 32 bit data length on 32 bit wide channel
    Data32Channel32,
}

impl Default for DataFormat {
    fn default() -> Self {
        DataFormat::Data16Channel16
    }
}

#[derive(Debug, Clone, Copy)]
/// I2s driver configuration. Can be used as an i2s driver builder.
///
///  - `MS`: `Master` or `Slave`
///  - `TR`: `Transmit` or `Receive`
///  - `STD`: I2S standard, eg `Philips`
///
/// **Note:** because of it's typestate, methods of this type don't change variable content, they
/// return a new value instead.
pub struct I2sDriverConfig<MS, TR, STD> {
    slave_or_master: SlaveOrMaster,
    transmit_or_receive: TransmitOrReceive,
    standard: I2sStandard,
    clock_polarity: ClockPolarity,
    data_format: DataFormat,
    master_clock: bool,
    frequency: Frequency,

    _ms: PhantomData<MS>,
    _tr: PhantomData<TR>,
    _std: PhantomData<STD>,
}

impl I2sDriverConfig<Slave, Transmit, Philips> {
    /// Create a new default slave configuration.
    pub fn new_slave() -> Self {
        Self {
            slave_or_master: SlaveOrMaster::Slave,
            transmit_or_receive: TransmitOrReceive::Transmit,
            standard: I2sStandard::Philips,
            clock_polarity: ClockPolarity::IdleLow,
            data_format: Default::default(),
            master_clock: false,
            frequency: Frequency::Prescaler(false, 0b10),
            _ms: PhantomData,
            _tr: PhantomData,
            _std: PhantomData,
        }
    }
}

impl I2sDriverConfig<Master, Transmit, Philips> {
    /// Create a new default master configuration.
    pub fn new_master() -> Self {
        Self {
            slave_or_master: SlaveOrMaster::Master,
            transmit_or_receive: TransmitOrReceive::Transmit,
            standard: I2sStandard::Philips,
            clock_polarity: ClockPolarity::IdleLow,
            data_format: Default::default(),
            master_clock: false,
            frequency: Frequency::Prescaler(false, 0b10),
            _ms: PhantomData,
            _tr: PhantomData,
            _std: PhantomData,
        }
    }
}

/// rounding division
fn div_round(n: u32, d: u32) -> u32 {
    (n + (d >> 1)) / d
}

// unsafe, div should be greater or equal to 2
fn _set_prescaler(w: &mut i2spr::W, odd: bool, div: u8) {
    w.odd().bit(odd);
    unsafe { w.i2sdiv().bits(div) };
}

// Note, calculation details:
// Fs = i2s_clock / [256 * ((2 * div) + odd)] when master clock is enabled
// Fs = i2s_clock / [(channel_length * 2) * ((2 * div) + odd)]` when master clock is disabled
// channel_length is 16 or 32
//
// can be rewritten as
// Fs = i2s_clock / (coef * division)
// where coef is a constant equal to 256, 64 or 32 depending channel length and master clock
// and where division = (2 * div) + odd
//
// Equation can be rewritten as
// division = i2s_clock/ (coef * Fs)
//
// note: division = (2 * div) + odd = (div << 1) + odd
// in other word, from bits point of view, division[8:1] = div[7:0] and division[0] = odd
fn _set_request_frequency(
    w: &mut i2spr::W,
    i2s_clock: u32,
    request_freq: u32,
    mclk: bool,
    data_format: DataFormat,
) {
    let coef = _coef(mclk, data_format);
    let division = div_round(i2s_clock, coef * request_freq);
    let (odd, div) = if division < 4 {
        (false, 2)
    } else if division > 511 {
        (true, 255)
    } else {
        ((division & 1) == 1, (division >> 1) as u8)
    };
    _set_prescaler(w, odd, div);
}

// see _set_request_frequency for explanation
fn _set_require_frequency(
    w: &mut i2spr::W,
    i2s_clock: u32,
    request_freq: u32,
    mclk: bool,
    data_format: DataFormat,
) {
    let coef = _coef(mclk, data_format);
    let division = i2s_clock / (coef * request_freq);
    let rem = i2s_clock / (coef * request_freq);
    if rem == 0 && division >= 4 && division <= 511 {
        let odd = (division & 1) == 1;
        let div = (division >> 1) as u8;
        _set_prescaler(w, odd, div);
    } else {
        panic!("Cannot reach exactly the required frequency")
    };
}

// see _set_request_frequency for explanation
fn _coef(mclk: bool, data_format: DataFormat) -> u32 {
    if mclk {
        return 256;
    }
    if let DataFormat::Data16Channel16 = data_format {
        32
    } else {
        64
    }
}

impl<MS, TR, STD> I2sDriverConfig<MS, TR, STD> {
    /// Instantiate the driver by wrapping the given [`I2sPeripheral`].
    ///
    /// # Panics
    ///
    /// This method panics if an exact frequency is required and that frequency cannot be set.
    pub fn i2s_driver<I: I2sPeripheral>(self, i2s_peripheral: I) -> I2sDriver<I, MS, TR, STD> {
        let driver = I2sDriver::<I, MS, TR, STD> {
            i2s_peripheral,
            _ms: PhantomData,
            _tr: PhantomData,
            _std: PhantomData,
        };
        driver.registers().cr1.reset(); // ensure SPI is disabled
        driver.registers().cr2.reset(); // disable interrupt and DMA request
        driver.registers().i2scfgr.write(|w| {
            w.i2smod().i2smode();
            match (self.slave_or_master, self.transmit_or_receive) {
                (SlaveOrMaster::Slave, TransmitOrReceive::Transmit) => w.i2scfg().slave_tx(),
                (SlaveOrMaster::Slave, TransmitOrReceive::Receive) => w.i2scfg().slave_rx(),
                (SlaveOrMaster::Master, TransmitOrReceive::Transmit) => w.i2scfg().master_tx(),
                (SlaveOrMaster::Master, TransmitOrReceive::Receive) => w.i2scfg().master_rx(),
            };
            match self.standard {
                I2sStandard::Philips => w.i2sstd().philips(),
                I2sStandard::Msb => w.i2sstd().msb(),
                I2sStandard::Lsb => w.i2sstd().lsb(),
                I2sStandard::PcmShortSync => w.i2sstd().pcm().pcmsync().short(),
                I2sStandard::PcmLongSync => w.i2sstd().pcm().pcmsync().long(),
            };
            match self.data_format {
                DataFormat::Data16Channel16 => w.datlen().sixteen_bit().chlen().sixteen_bit(),
                DataFormat::Data16Channel32 => w.datlen().sixteen_bit().chlen().thirty_two_bit(),
                DataFormat::Data24Channel32 => {
                    w.datlen().twenty_four_bit().chlen().thirty_two_bit()
                }
                DataFormat::Data32Channel32 => w.datlen().thirty_two_bit().chlen().thirty_two_bit(),
            };
            w
        });
        driver.registers().i2spr.write(|w| {
            w.mckoe().bit(self.master_clock);
            match self.frequency {
                Frequency::Prescaler(odd, div) => _set_prescaler(w, odd, div),
                Frequency::Request(freq) => _set_request_frequency(
                    w,
                    driver.i2s_peripheral.i2s_freq(),
                    freq,
                    self.master_clock,
                    self.data_format,
                ),
                Frequency::Require(freq) => _set_require_frequency(
                    w,
                    driver.i2s_peripheral.i2s_freq(),
                    freq,
                    self.master_clock,
                    self.data_format,
                ),
            }
            w
        });
        driver
    }
}

impl Default for I2sDriverConfig<Slave, Transmit, Philips> {
    /// Create a default configuration. It correspond to a default slave configuration.
    fn default() -> Self {
        Self::new_slave()
    }
}

impl<MS, TR, STD> I2sDriverConfig<MS, TR, STD> {
    /// Configure driver in transmit mode
    pub fn transmit(self) -> I2sDriverConfig<MS, Transmit, STD> {
        I2sDriverConfig::<MS, Transmit, STD> {
            slave_or_master: self.slave_or_master,
            transmit_or_receive: TransmitOrReceive::Transmit,
            standard: self.standard,
            clock_polarity: self.clock_polarity,
            data_format: self.data_format,
            master_clock: self.master_clock,
            frequency: self.frequency,
            _ms: PhantomData,
            _tr: PhantomData,
            _std: PhantomData,
        }
    }
    /// Configure driver in receive mode
    pub fn receive(self) -> I2sDriverConfig<MS, Receive, STD> {
        I2sDriverConfig::<MS, Receive, STD> {
            slave_or_master: self.slave_or_master,
            transmit_or_receive: TransmitOrReceive::Receive,
            standard: self.standard,
            clock_polarity: self.clock_polarity,
            data_format: self.data_format,
            master_clock: self.master_clock,
            frequency: self.frequency,
            _ms: PhantomData,
            _tr: PhantomData,
            _std: PhantomData,
        }
    }
    /// Select the I2s standard to use
    #[allow(non_camel_case_types)]
    pub fn standard<NEW_STD>(self, _standard: NEW_STD) -> I2sDriverConfig<MS, TR, NEW_STD>
    where
        NEW_STD: marker::I2sStandard,
    {
        I2sDriverConfig::<MS, TR, NEW_STD> {
            slave_or_master: self.slave_or_master,
            transmit_or_receive: self.transmit_or_receive,
            standard: NEW_STD::VALUE,
            clock_polarity: self.clock_polarity,
            data_format: self.data_format,
            master_clock: self.master_clock,
            frequency: self.frequency,
            _ms: PhantomData,
            _tr: PhantomData,
            _std: PhantomData,
        }
    }
    /// Select steady state clock polarity
    // datasheet don't precise how it affect I2s operation. In particular, this may meaningless for
    // slave operation.
    pub fn clock_polarity(mut self, polarity: ClockPolarity) -> Self {
        self.clock_polarity = polarity;
        self
    }

    /// Select data format
    pub fn data_format(mut self, format: DataFormat) -> Self {
        self.data_format = format;
        self
    }

    /// Convert to a slave configuration. This delete Master Only Settings.
    pub fn to_slave(self) -> I2sDriverConfig<Slave, TR, STD> {
        let Self {
            transmit_or_receive,
            standard,
            clock_polarity,
            data_format,
            ..
        } = self;
        I2sDriverConfig::<Slave, TR, STD> {
            slave_or_master: SlaveOrMaster::Slave,
            transmit_or_receive,
            standard,
            clock_polarity,
            data_format,
            master_clock: false,
            frequency: Frequency::Prescaler(false, 0b10),
            _ms: PhantomData,
            _tr: PhantomData,
            _std: PhantomData,
        }
    }

    /// Convert to a master configuration.
    pub fn to_master(self) -> I2sDriverConfig<Master, TR, STD> {
        let Self {
            transmit_or_receive,
            standard,
            clock_polarity,
            data_format,
            master_clock,
            frequency,
            ..
        } = self;
        I2sDriverConfig::<Master, TR, STD> {
            slave_or_master: SlaveOrMaster::Master,
            transmit_or_receive,
            standard,
            clock_polarity,
            data_format,
            master_clock,
            frequency,
            _ms: PhantomData,
            _tr: PhantomData,
            _std: PhantomData,
        }
    }
}

impl<TR, STD> I2sDriverConfig<Master, TR, STD> {
    /// Enable/Disable Master Clock. Affect the effective sampling rate.
    ///
    /// This can be only set and only have meaning for Master mode.
    pub fn master_clock(mut self, enable: bool) -> Self {
        self.master_clock = enable;
        self
    }

    /// Configure audio frequency by setting the prescaler with an odd factor and a divider.
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
    pub fn prescaler(mut self, odd: bool, div: u8) -> Self {
        #[allow(clippy::manual_range_contains)]
        if div < 2 {
            panic!("div is less than 2, forbidden value")
        }
        self.frequency = Frequency::Prescaler(odd, div);
        self
    }

    /// Request an audio sampling frequency. The effective audio sampling frequency may differ.
    pub fn request_frequency(mut self, freq: u32) -> Self {
        self.frequency = Frequency::Request(freq);
        self
    }

    /// Require exactly this audio sampling frequency.
    ///
    /// If the required frequency can not bet set, Instantiate the driver will panics.
    pub fn require_frequency(mut self, freq: u32) -> Self {
        self.frequency = Frequency::Require(freq);
        self
    }
}

/// Driver of a SPI peripheral in I2S mode.
///
/// Meant for advanced usage, for example using interrupt or DMA.
pub struct I2sDriver<I, MS, TR, STD> {
    i2s_peripheral: I,
    _ms: PhantomData<MS>,
    _tr: PhantomData<TR>,
    _std: PhantomData<STD>,
}

impl<I, MS, TR, STD> I2sDriver<I, MS, TR, STD>
where
    I: I2sPeripheral,
{
    /// Returns a reference to the register block
    fn registers(&self) -> &RegisterBlock {
        unsafe { &*(I::REGISTERS as *const RegisterBlock) }
    }
}

/// Constructors and Destructors
impl<I, MS, TR, STD> I2sDriver<I, MS, TR, STD>
where
    I: I2sPeripheral,
{
    /// Instantiate an i2s driver from an [`I2sPeripheral`] object and a configuration.
    ///
    /// # Panics
    ///
    /// This method panics if an exact frequency is required by the configuration and that
    /// frequency can not be set.
    pub fn new(i2s_peripheral: I, config: I2sDriverConfig<MS, TR, STD>) -> Self {
        config.i2s_driver(i2s_peripheral)
    }

    /// Destroy the driver, release and reset the owned i2s device.
    pub fn release(mut self) -> I {
        self.i2s_peripheral.rcc_reset();
        self.i2s_peripheral
    }

    /// Consume the driver and create a new one with the given configuration.
    #[allow(non_camel_case_types)]
    pub fn reconfigure<NEW_MS, NEW_TR, NEW_STD>(
        self,
        config: I2sDriverConfig<NEW_MS, NEW_TR, NEW_STD>,
    ) -> I2sDriver<I, NEW_MS, NEW_TR, NEW_STD> {
        let i2s_peripheral = self.i2s_peripheral;
        config.i2s_driver(i2s_peripheral)
    }
}

/// Methods avaible in any mode
impl<I, MS, TR, STD> I2sDriver<I, MS, TR, STD>
where
    I: I2sPeripheral,
{
    /// Enable the I2S peripheral.
    pub fn enable(&mut self) {
        self.registers().i2scfgr.modify(|_, w| w.i2se().enabled());
    }

    /// Immediately Disable the I2S peripheral. Generated clocks aren't reseted so a call to
    /// `reset_clocks` may required in master mode.
    ///
    /// It's up to the caller to not disable the peripheral in the middle of a frame.
    pub fn disable(&mut self) {
        self.registers().i2scfgr.modify(|_, w| w.i2se().disabled());
    }

    /// Return `true` if the level on the WS line is high.
    #[deprecated(
        since = "0.4.0",
        note = "may removed in future, use `ws_pin().is_high()` instead"
    )]
    pub fn ws_is_high(&self) -> bool {
        self.i2s_peripheral.ws_pin().is_high()
    }

    /// Return `true` if the level on the WS line is low.
    #[deprecated(
        since = "0.4.0",
        note = "may removed in future, use `ws_pin().is_low()` instead"
    )]
    pub fn ws_is_low(&self) -> bool {
        self.i2s_peripheral.ws_pin().is_low()
    }

    /// Get a reference to the WS pin.
    pub fn ws_pin(&self) -> &I::WsPin {
        self.i2s_peripheral.ws_pin()
    }

    /// Get a mutable reference to the WS pin.
    pub fn ws_pin_mut(&mut self) -> &mut I::WsPin {
        self.i2s_peripheral.ws_pin_mut()
    }

    /// Get address of data register for dma setup.
    pub fn data_register_address(&self) -> u32 {
        &(self.registers().dr) as *const _ as u32
    }
    /// Get the content of the status register. It's content may modified during the operation.
    ///
    /// When reading the status register, the hardware may reset some error flag of it. The way
    /// each flag can be modified is documented on each [Status] flag getter.
    pub fn status(&mut self) -> Status<MS, TR, STD> {
        Status::<MS, TR, STD> {
            value: self.registers().sr.read(),
            _ms: PhantomData,
            _tr: PhantomData,
            _std: PhantomData,
        }
    }
}

/// Master only methods
impl<I, TR, STD> I2sDriver<I, Master, TR, STD>
where
    I: I2sPeripheral,
{
    /// Reset clocks generated by the peripheral. Also delete status and data registers.
    ///
    /// This allow to immediately start a new frame when an error occurred or before enabling again
    /// the driver.
    pub fn reset_clocks(&mut self) {
        let registers = self.registers();
        let cr2 = registers.cr2.read().bits();
        let i2scfgr = registers.i2scfgr.read().bits();
        let i2spr = registers.i2spr.read().bits();
        self.i2s_peripheral.rcc_reset();
        let registers = self.registers();
        registers.cr2.write(|w| unsafe { w.bits(cr2) });
        registers.i2spr.write(|w| unsafe { w.bits(i2spr) });
        registers.i2scfgr.write(|w| unsafe { w.bits(i2scfgr) });
    }

    /// Get the actual sample rate imposed by the driver.
    ///
    /// This allow to check deviation with a requested frequency.
    pub fn sample_rate(&self) -> u32 {
        let i2spr = self.registers().i2spr.read();
        let mckoe = i2spr.mckoe().bit();
        let odd = i2spr.odd().bit();
        let div = i2spr.i2sdiv().bits();
        let i2s_freq = self.i2s_peripheral.i2s_freq();
        if mckoe {
            i2s_freq / (256 * ((2 * div as u32) + odd as u32))
        } else {
            match self.registers().i2scfgr.read().chlen().bit() {
                false => i2s_freq / ((16 * 2) * ((2 * div as u32) + odd as u32)),
                true => i2s_freq / ((32 * 2) * ((2 * div as u32) + odd as u32)),
            }
        }
    }
}

/// Transmit only methods
impl<I, MS, STD> I2sDriver<I, MS, Transmit, STD>
where
    I: I2sPeripheral,
{
    /// Write a raw half word to the Tx buffer and delete the TXE flag in status register.
    ///
    /// It's up to the caller to write the content when it's empty.
    pub fn write_data_register(&mut self, value: u16) {
        self.registers().dr.write(|w| w.dr().bits(value));
    }

    /// When set to `true`, an interrupt is generated each time the Tx buffer is empty.
    pub fn set_tx_interrupt(&mut self, enabled: bool) {
        self.registers().cr2.modify(|_, w| w.txeie().bit(enabled))
    }

    /// When set to `true`, a dma request is generated each time the Tx buffer is empty.
    pub fn set_tx_dma(&mut self, enabled: bool) {
        self.registers().cr2.modify(|_, w| w.txdmaen().bit(enabled))
    }
}

/// Receive only methods
impl<I, MS, STD> I2sDriver<I, MS, Receive, STD>
where
    I: I2sPeripheral,
{
    /// Read a raw value from the Rx buffer and delete the RXNE flag in status register.
    pub fn read_data_register(&mut self) -> u16 {
        self.registers().dr.read().dr().bits()
    }

    /// When set to `true`, an interrupt is generated each time the Rx buffer contains a new data.
    pub fn set_rx_interrupt(&mut self, enabled: bool) {
        self.registers().cr2.modify(|_, w| w.rxneie().bit(enabled))
    }

    /// When set to `true`, a dma request is generated each time the Rx buffer contains a new data.
    pub fn set_rx_dma(&mut self, enabled: bool) {
        self.registers().cr2.modify(|_, w| w.rxdmaen().bit(enabled))
    }
}

/// Error interrupt, Master Receive Mode.
impl<I, STD> I2sDriver<I, Master, Receive, STD>
where
    I: I2sPeripheral,
{
    /// When set to `true`, an interrupt is generated each time an error occurs.
    ///
    /// Not available for Master Transmit because no error can occur in this mode.
    pub fn set_error_interrupt(&mut self, enabled: bool) {
        self.registers().cr2.modify(|_, w| w.errie().bit(enabled))
    }
}

/// Error interrupt, Slave Mode.
impl<I, TR, STD> I2sDriver<I, Slave, TR, STD>
where
    I: I2sPeripheral,
{
    /// When set to `true`, an interrupt is generated each time an error occurs.
    ///
    /// Not available for Master Transmit because no error can occur in this mode.
    pub fn set_error_interrupt(&mut self, enabled: bool) {
        self.registers().cr2.modify(|_, w| w.errie().bit(enabled))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_div_round() {
        let fracs = [(1, 2), (2, 2), (1, 3), (2, 3), (2, 4), (3, 5), (9, 2)];
        for (n, d) in fracs {
            let res = div_round(n, d);
            let check = f32::round((n as f32) / (d as f32)) as u32;
            assert_eq!(res, check);
        }
    }
}
