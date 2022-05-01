//! This library supports I2S communication for SPI version 1.2 (on STM32F1, STM32F2, STM32F4,
//! STM32L0, and STM32L1 microcontrollers).
//!
//! This library is normally used with a HAL library that provides a type that implements
//! [I2sPeripheral](crate::I2sPeripheral). An [I2sDriver](crate::I2sDriver) object can be created around the I2sPeripheral
//! object and used for I2S.

#![no_std]

extern crate nb;
extern crate vcell;

//mod config;
mod pac;

mod sealed {
    pub trait Sealed {}
}
//use self::sealed::Sealed;

use core::marker::PhantomData;

//pub use self::config::{MasterClock, MasterConfig, SlaveConfig};
use self::pac::spi1::RegisterBlock;
use self::pac::spi1::{i2spr, sr};
//use crate::format::{DataFormat, FrameFormat, FrameSync};
//use crate::pac::spi1::i2scfgr::I2SCFG_A;

/// Marker, indicated master mode.
#[derive(Debug, Clone, Copy)]
pub struct Master;

/// Marker, indicate slave mode.
#[derive(Debug, Clone, Copy)]
pub struct Slave;

/// The channel associated with a sample
#[derive(Debug, Clone, PartialEq)]
pub enum Channel {
    /// Left channel (word select low)
    Left,
    /// Right channel (word select high)
    Right,
}

/// Content of the status register.
pub struct Status {
    value: sr::R,
}

impl Status {
    /// Get the FRE flag. If `true` a frame error occured.
    ///
    /// This flag can be set by hardware only if the I2sDriver is configured in Slave mode. It is set
    /// when the WS line change at an unexpected moment. Usually, this indicate a synchronisation
    /// issue. This flag is cleared when reading the status register.
    pub fn fre(&self) -> bool {
        self.value.fre().bit()
    }

    /// Get the BSY flag. If `true` the I2s device is busy communicating.
    pub fn bsy(&self) -> bool {
        self.value.bsy().bit()
    }

    /// Get the OVR flag. If `true` an overrun error occured.
    ///
    /// This flag is set when data are received and the previous data have not yet been read. As a
    /// result, the incoming data are lost. This flag is cleared by a read operation on the data
    /// register followed by a read to the status register.
    pub fn ovr(&self) -> bool {
        self.value.ovr().bit()
    }

    /// Get the UDR flag. If `true` an underrun error occured.
    ///
    /// This flag can be set only in slave transmission mode. It is set when the first clock for
    /// data transmission appears while the software has not yet loaded any value into the data
    /// register.
    /// This flag is cleared by reading the status register.
    pub fn udr(&self) -> bool {
        self.value.udr().bit()
    }

    /// Get the CHSIDE flag. It indicate the channel has been received or to be transmitted. Have
    /// no meaning with PCM standard.
    ///
    /// This flag is updated when TXE or RXNE flags are set. This flag is meaningless and therefore
    /// not reliable is case of error or when using the PCM standard.
    pub fn chside(&self) -> Channel {
        match self.value.udr().bit() {
            false => Channel::Left,
            true => Channel::Right,
        }
    }

    /// Get the TXE flag. If `true` the Tx buffer is empty and the next data can be loaded into it.
    ///
    /// This flag can be set only in transmision mode. This flag is cleared by writing into the
    /// data register or by disabling the I2s peripheral.
    pub fn txe(&self) -> bool {
        self.value.txe().bit()
    }

    /// Get the RXNE flag. If `true` a valid received data is present in the Rx buffer.
    ///
    /// This flag can be only set in reception mode. It is cleared when the data register is read.
    pub fn rxne(&self) -> bool {
        self.value.rxne().bit()
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
    /// 16 bit date length on 16 bit wide channel
    Data16Channel16,
    /// 16 bit date length on 32 bit wide channel
    Data16Channel32,
    /// 24 bit date length on 32 bit wide channel
    Data24Channel32,
    /// 32 bit date length on 32 bit wide channel
    Data32Channel32,
}

impl Default for DataFormat {
    fn default() -> Self {
        DataFormat::Data16Channel16
    }
}

#[derive(Debug, Clone, Copy)]
/// I2s Configuration builder.
pub struct Config<MS> {
    slave_or_master: SlaveOrMaster,
    transmit_or_receive: TransmitOrReceive,
    standard: I2sStandard,
    clock_polarity: ClockPolarity,
    data_format: DataFormat,
    master_clock: bool,
    frequency: Frequency,

    _ms: PhantomData<MS>,
}

impl Config<Slave> {
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
        }
    }
}

impl Config<Master> {
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

// set _set_request_frequency for explanation
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

impl<MS> Config<MS> {
    /// Instantiate the driver.
    pub fn i2s_driver<I: I2sPeripheral>(self, i2s_peripheral: I) -> I2sDriver<I> {
        let driver = I2sDriver { i2s_peripheral };
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

impl Default for Config<Slave> {
    /// Create a default configuration. It correspond to a default slave configuration.
    fn default() -> Self {
        Self::new_slave()
    }
}

impl<MS> Config<MS> {
    /// Configure in transmit mode
    pub fn transmit(mut self) -> Self {
        self.transmit_or_receive = TransmitOrReceive::Transmit;
        self
    }
    /// Configure in transmit mode
    pub fn receive(mut self) -> Self {
        self.transmit_or_receive = TransmitOrReceive::Receive;
        self
    }
    /// Select the I2s standard to use
    pub fn standard(mut self, standard: I2sStandard) -> Self {
        self.standard = standard;
        self
    }
    /// Select steady state clock polarity
    // datasheet don't precise how it affect I2s operation. In particular, this may meaningless for
    // slave operation.
    pub fn clock_polarity(mut self, polarity: ClockPolarity) -> Self {
        self.clock_polarity = polarity;
        self
    }

    /// Convert to a slave configuration. This delete Master Only Settings.
    pub fn to_slave(self) -> Config<Slave> {
        let Self {
            slave_or_master,
            transmit_or_receive,
            standard,
            clock_polarity,
            data_format,
            ..
        } = self;
        Config::<Slave> {
            slave_or_master,
            transmit_or_receive,
            standard,
            clock_polarity,
            data_format,
            master_clock: false,
            frequency: Frequency::Prescaler(false, 0b10),
            _ms: PhantomData,
        }
    }

    /// Convert to a master configuration.
    pub fn to_master(self) -> Config<Slave> {
        let Self {
            slave_or_master,
            transmit_or_receive,
            standard,
            clock_polarity,
            data_format,
            master_clock,
            frequency,
            ..
        } = self;
        Config::<Slave> {
            slave_or_master,
            transmit_or_receive,
            standard,
            clock_polarity,
            data_format,
            master_clock,
            frequency,
            _ms: PhantomData,
        }
    }
}

impl Config<Master> {
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
            panic!("div is less than 2, frobidden value")
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
    /// If the required frequency can not bet set, Instatiate the driver will produce a error
    pub fn require_frequency(mut self, freq: u32) -> Self {
        self.frequency = Frequency::Require(freq);
        self
    }
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
    /// Pointer to the SPI register block
    const REGISTERS: *const ();
    /// Get I2s clock source frequency from the I2s device.
    ///
    /// Implemetors are allowed to panic in case i2s source frequencey is unavailable.
    fn i2s_freq(&self) -> u32;
    /// Return `true` if the level at WS pin is high.
    fn ws_is_high(&self) -> bool;
    /// Return `true` if the level at WS pin is low.
    fn ws_is_low(&self) -> bool;
}

/// Driver of a SPI peripheral in I2S mode
///
/// # Example
///
/// TODO
///
/// ```no_run
/// ```
///
pub struct I2sDriver<I> {
    i2s_peripheral: I,
}

impl<I> I2sDriver<I>
where
    I: I2sPeripheral,
{
    /// Returns a reference to the register block
    fn registers(&self) -> &RegisterBlock {
        unsafe { &*(I::REGISTERS as *const RegisterBlock) }
    }
}

/// Constructors and Desctructors
impl<I> I2sDriver<I>
where
    I: I2sPeripheral,
{
    /// Instantiate and configure an i2s driver.
    pub fn new<MS>(i2s_peripheral: I, config: Config<MS>) -> I2sDriver<I> {
        config.i2s_driver(i2s_peripheral)
    }

    /// Destroy the driver, release the owned i2s device and reset it's configuration.
    pub fn release(self) -> I {
        let registers = self.registers();
        registers.cr1.reset();
        registers.cr2.reset();
        registers.i2scfgr.reset();
        registers.i2spr.reset();
        self.i2s_peripheral
    }

    /// Consume the driver and create a new one with the given config
    pub fn reconfigure<MS>(self, config: Config<MS>) -> I2sDriver<I> {
        let i2s_peripheral = self.i2s_peripheral;
        config.i2s_driver(i2s_peripheral)
    }
}

impl<I> I2sDriver<I>
where
    I: I2sPeripheral,
{
    /// Get a reference to the underlaying i2s device
    pub fn i2s_peripheral(&self) -> &I {
        &self.i2s_peripheral
    }

    /// Enable the I2S peripheral.
    pub fn enable(&mut self) {
        self.registers().i2scfgr.modify(|_, w| w.i2se().enabled());
    }

    /// Immediately Disable the I2S peripheral.
    ///
    /// It's up to the caller to not disable the peripheral in the middle of a frame.
    pub fn disable(&mut self) {
        self.registers().i2scfgr.modify(|_, w| w.i2se().disabled());
    }

    /// Get the content of the status register. It's content may modified during the operation.
    ///
    /// When reading the status register, the hardware may reset some error flag of it. The way
    /// each flag can be modified is documented on each [Status] flag getter.
    pub fn status(&mut self) -> Status {
        Status {
            value: self.registers().sr.read(),
        }
    }

    /// Write a raw half word to the Tx buffer and delete the TXE flag in status register.
    ///
    /// It's up to the caller to write the content when it's empty.
    pub fn write_data_register(&mut self, value: u16) {
        self.registers().dr.write(|w| w.dr().bits(value));
    }

    /// Read a raw value from the Rx buffer and delete the RXNE flag in status register.
    pub fn read_data_register(&mut self) -> u16 {
        self.registers().dr.read().dr().bits()
    }

    /// When set to `true`, an interrupt is generated each time the Tx buffer is empty.
    pub fn set_tx_interrupt(&mut self, enabled: bool) {
        self.registers().cr2.modify(|_, w| w.txeie().bit(enabled))
    }

    /// When set to `true`, an interrupt is generated each time the Rx buffer contains a new data.
    pub fn set_rx_interrupt(&mut self, enabled: bool) {
        self.registers().cr2.modify(|_, w| w.rxneie().bit(enabled))
    }

    /// When set to `true`, an interrupt is generated each time an error occurs.
    pub fn set_error_interrupt(&mut self, enabled: bool) {
        self.registers().cr2.modify(|_, w| w.errie().bit(enabled))
    }

    /// When set to `true`, a dma request is generated each time the Tx buffer is empty.
    pub fn set_tx_dma(&mut self, enabled: bool) {
        self.registers().cr2.modify(|_, w| w.txdmaen().bit(enabled))
    }

    /// When set to `true`, a dma request is generated each time the Rx buffer contains a new data.
    pub fn set_rx_dma(&mut self, enabled: bool) {
        self.registers().cr2.modify(|_, w| w.rxdmaen().bit(enabled))
    }

    /// Return `true` if the level on the WS line is high.
    pub fn ws_is_high(&self) -> bool {
        self.i2s_peripheral.ws_is_high()
    }

    /// Return `true` if the level on the WS line is low.
    pub fn ws_is_low(&self) -> bool {
        self.i2s_peripheral.ws_is_low()
    }

    //TODO method to get a handle to WS pin. It may usefull for setting an interrupt on pin to
    //synchronise I2s in slave mode
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_div_rounding() {
        let fracs = [(1, 2), (2, 2), (1, 3), (2, 3), (2, 4), (3, 5), (9, 2)];
        for (n, d) in fracs {
            let res = div_rounding(n, d);
            let check = f32::round((n as f32) / (d as f32)) as u32;
            assert_eq!(res, check);
        }
    }
}
