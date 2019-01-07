use super::super::utxo::UTxO;
use super::{Address, AddressLookup, AddressLookupError};
use cardano::address::ExtendedAddr;

pub struct Accum();
impl Default for Accum {
    fn default() -> Self {
        Accum()
    }
}

impl AddressLookup for Accum {
    fn lookup(
        &mut self,
        utxo: UTxO<ExtendedAddr>,
    ) -> Result<Option<UTxO<Address>>, AddressLookupError> {
        Ok(Some(utxo.map(|a| a.into())))
    }

    fn acknowledge<A>(&mut self, _: A) -> Result<(), AddressLookupError>
    where
        A: Into<Address>,
    {
        Ok(())
    }
}
