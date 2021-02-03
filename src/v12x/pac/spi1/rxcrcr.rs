#[doc = "Reader of register RXCRCR"]
pub type R = crate::v12x::pac::R<u32, super::RXCRCR>;
#[doc = "Reader of field `RxCRC`"]
pub type RXCRC_R = crate::v12x::pac::R<u16, u16>;
impl R {
    #[doc = "Bits 0:15 - Rx CRC register"]
    #[inline(always)]
    pub fn rx_crc(&self) -> RXCRC_R {
        RXCRC_R::new((self.bits & 0xffff) as u16)
    }
}
