//!
//! Periodically transmits a sequence of 24-bit samples (extended into 32-bit frames)
//! using SPI1/I2S1 on an STM32F412
//!
//! Pins:
//! * PA4, AF5 - I2S1_WS
//! * PA5, AF5 - I2S1_CK
//! * PA7, AF5 - I2S1_SD
//!
//! To compile:
//! RUSTFLAGS="-C link-arg=-Tlink.x" cargo build --example master_transmit_d24f32 --target thumbv7em-none-eabihf --release
//!
//! This uses some unsafe code so that it can depend on a version of stm32f4xx-hal that does not
//! depend on stm32_i2s.
//!

#![no_std]
#![no_main]

extern crate cortex_m_rt;
extern crate nb;
extern crate panic_rtt_target;
extern crate rtt_target;
extern crate stm32_i2s;
extern crate stm32f4xx_hal;

use stm32f4xx_hal::hal::prelude::*;
use stm32f4xx_hal::pac::{CorePeripherals, Peripherals};
use stm32f4xx_hal::prelude::*;
use stm32f4xx_hal::timer::Timer;

use stm32_i2s::v12x::format::{Data24Frame32, FrameFormat};
use stm32_i2s::v12x::{I2s, Instance, MasterConfig, RegisterBlock};
use stm32_i2s::Polarity;

/// 16-bit samples to transmit
const TEST_SAMPLES: [i32; 12] = [
    0x00_0000,
    0x00_0000,
    0x20_aa55_u32 as i32,
    0x26_55aa_u32 as i32,
    0x01_0000_u32 as i32,
    0x99_ffff_u32 as i32,
    0xe9_1010_u32 as i32,
    0xf3_aaaa_u32 as i32,
    0xcd_5555_u32 as i32,
    0xe9_e621_u32 as i32,
    0x00_0000,
    0x00_0000,
];

/// Sample rates to test
const SAMPLE_RATES: [u32; 8] = [8000, 16000, 22050, 32000, 44100, 48000, 96000, 192000];

#[cortex_m_rt::entry]
fn main() -> ! {
    let cp = CorePeripherals::take().unwrap();
    let dp = Peripherals::take().unwrap();
    // RTT for logging
    rtt_target::rtt_init_print!();

    let rcc = dp.RCC.constrain();
    // SPI1/I2S1 is on APB2
    let clocks = rcc
        .cfgr
        .sysclk(100.mhz())
        .i2s_apb1_clk(76800.khz())
        .i2s_apb2_clk(76800.khz())
        .freeze();

    // Use systick to run periodically
    let mut systick = Timer::syst(cp.SYST, 1000.hz(), clocks);

    let gpioa = dp.GPIOA.split();
    let _i2s_pins = (
        gpioa.pa4.into_alternate_af5(),
        gpioa.pa5.into_alternate_af5(),
        gpioa.pa7.into_alternate_af5(),
    );
    let mut sync_pin = gpioa.pa1.into_push_pull_output();
    sync_pin.set_low().unwrap();

    // Access the RCC registers directly to enable SPI1
    unsafe {
        let rcc_registers = stm32f4xx_hal::pac::RCC::ptr();
        (*rcc_registers).apb2enr.modify(|_, w| w.spi1en().enabled());
    }

    let mut i2s = I2s::new(I2s1Substitute);

    loop {
        for &sample_rate in SAMPLE_RATES.iter() {
            let config = MasterConfig::with_sample_rate(
                clocks.i2s_apb2_clk().unwrap().0,
                sample_rate,
                Data24Frame32,
                FrameFormat::PhilipsI2s,
                Polarity::IdleHigh,
                false,
            );
            let mut configured_i2s = i2s.configure_master_transmit(config);

            configured_i2s.enable();
            configured_i2s.transmit_blocking(&TEST_SAMPLES);
            nb::block!(configured_i2s.disable()).unwrap();

            i2s = configured_i2s.deconfigure();

            nb::block!(systick.wait()).unwrap();
        }
        sync_pin.toggle().unwrap();
    }
}

struct I2s1Substitute;

unsafe impl Instance for I2s1Substitute {
    const REGISTERS: *mut RegisterBlock = stm32f4xx_hal::pac::SPI1::ptr() as *mut RegisterBlock;
}
