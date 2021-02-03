//!
//!
//!

#![no_std]

extern crate nb;
extern crate vcell;

pub mod v12x;

/// Clock polarity
#[derive(Debug, Clone)]
pub enum Polarity {
    /// Clock low when idle
    IdleLow,
    /// Clock high when idle
    IdleHigh,
}

/// The channel associated with a sample
#[derive(Debug, Clone, PartialEq)]
pub enum Channel {
    /// Left channel (word select low)
    Left,
    /// Right channel (word select high)
    Right,
}
