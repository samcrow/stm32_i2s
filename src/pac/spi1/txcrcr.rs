#[doc = "Reader of register TXCRCR"]
pub type R = crate::pac::R<u32, super::TXCRCR>;
#[doc = "Reader of field `TxCRC`"]
pub type TXCRC_R = crate::pac::R<u16, u16>;
impl R {
    #[doc = "Bits 0:15 - Tx CRC register"]
    #[inline(always)]
    pub fn tx_crc(&self) -> TXCRC_R {
        TXCRC_R::new((self.bits & 0xffff) as u16)
    }
}
