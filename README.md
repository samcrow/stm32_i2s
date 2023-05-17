# STM32 I2S driver

This library provides driver and abstractions for I2S communication using the
SPI peripherals on some STM32 microcontrollers.

## Differences between STM32 models

According to application note
[AN5543](https://www.st.com/resource/en/application_note/dm00725181-enhanced-methods-to-handle-spi-communication-on-stm32-devices-stmicroelectronics.pdf),
there are four major versions of the SPI/I2S peripheral used on STM32
microcontrollers:

* 1.2.x: F1, F2, F4, L0, L1
* 1.3.x: F0, F3, F7, L4, L5, WB, WL
* 2.x.x: H7, MP1
* 3.x.x: "Most of STM32 devices launched in 2021 or later"

Currently, code of this library is focused on SPI version 1.2 (STM32F1,
STM32F2, STM32F4, STM32L0, and STM32L1). However, SPI version 1.2 and 1.3 don't
seems to have relevant differences for I2S operation and therefore this library
may also work with SPI version 1.3 (STM32F0 STM32F3 STM32F7 STM32L4
STM32L5...).

## Status

This library has been tested on a few different STM32F4 microcontrollers. The
other models that use the same SPI version (F1, F2, L0, and L1) may work, but
we haven't tested any of them. Trait implementations and a working example will be
availaible in [stm32f4xx-hal](https://github.com/stm32-rs/stm32f4xx-hal/).

## License

0-clause BSD (see LICENSE.txt)
