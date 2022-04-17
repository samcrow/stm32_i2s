# rework branch

Version 0.2 of this crate have many design flaw making it difficult to use in
general and even unusable in several situations. A redesign is required. The
new approach is to have a low-level driver exposing hardware specificity for
precise control and higher level abstraction will be built on top of this
driver, not inside it.

You can found below a list of important issues.

## Undocumented side effects

reading the status register can reset error flag.  Since many function access
to this register, this make impossible to get reliably those errors.

## Not meaningful and miss usable API

transmit/receive blocking a slice of interleaved samples: which is left which
is right ? First sample can be left or right depending what happened before.

`ready_to_transmit` and `sample_ready` return `Option<Channel>`. The `Channel`
information is meaningless in PCM mode.

## Missing feature

Slave operation require to read the WS pin to synchronise reliably. This
functionality is missing, so slave operation is near to unusable 






