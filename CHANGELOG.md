# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

### Added
 - DualI2sDriver allowing to use I2s extension notable for full duplex
   communication support
 - direction() function in I2sDriverConfig, for coherence with the new
   DualI2sDriverConfig
 - 'Direction' marker trait, implemented on Transmit and Receive marker to
   allow direction() function to work.
 - Information about possible SPI v1.3 compatibility in crate documentation

### Fixed
 - I2sDriver::sampling_rate() was unintentionally left unimplemented for PCM
   standards
 - Warning about CHSIDE flag in Master Transmit mode was accidentally removed
   in v0.4.0
 - Various errors and typos in documentation.

### Changed
 -  Generic parameters named '\*TR' are renamed '\*DIR'

## [v0.4.0 - 2023-04-01(https://github.com/samcrow/stm32_i2s/tree/v0.4.0)

## Added

- PCM support [#8](https://github.com/samcrow/stm32_i2s/pull/8)

## Changed

We don't have a user-friendly changelog, so here is a list of commits:

 
- 1512349 note about compatibility with SPI v 1.3
- f1f24b6 Merge pull request #8 from YruamaLairba/rcc_reset+pcm_fixsupport
- e4af332 update some documentations
- 32289e5 fix calculation for pcm case
- c94bd06 remove uneeded dependencies
- 565f276 improve documentation on some item, hide some details
- d30c4ee update module doc and fix examples
- d4ee221 update I2sTransfer documentation
- 856f3d9 Make Master Receive transfer fail on error with a coherent state
- 4809724 make sample rate caclulation unimplemented for pcm.
- cd77d77 tweak another macro
- f047ec9 tweak a macro to make invocation look better
- fe2e976 rename *_raw_frame function to *_raw
- 5608292 implment slave receive transfer
- 3eb01e1 FromRawFrame trait + Master receive transfer
- 5f66b9a generic master transmit transfer
- e15277f add sync and send trait bound to raw frame
- b50440a change internal representation of transfer, start to reimplement
- 396524c trick to get raw frame type from STD and FMT
- deba516 make reset_clock master only method, and reorganise code
- f9ce430 oops, some trait bound where wrong
- 2c6552d change master receive transfer to support pcm
- 82c6fec ending master transfer now reset the clock
- bc58ba4 warning about what disable does not
- 030ea45 add method to get data register address for dma usage
- 0a48b34 don't use deprecated stuff for transfer
- b3e3488 remove ws_is_* from trait, deprecate on driver
- 53280bb fix/update some doc
- b24c0da release now fully reset the i2s peripheral
- 0b2545d allow to get a WS Pin directly from driver
- 2dd7f5a remove direct access to peripheral to prevent typstate violation.
- a1b2ff7 reset_clock (untested)


## [v0.3.0 - 2022-06-19](https://github.com/samcrow/stm32_i2s/tree/v0.3.0)

### Changed

- Major restructure [#5](https://github.com/samcrow/stm32_i2s/pull/5)

## v0.2.0 - 2021-02-07

### Added

- Lower-level non-blocking functions to read and write the data register

## v0.1.0 - 2021-02-07

Initial release with basic functionality

