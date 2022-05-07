//! Markers for [`Config`](super::Config) and [`I2sDriver`](super::I2sDriver)

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
