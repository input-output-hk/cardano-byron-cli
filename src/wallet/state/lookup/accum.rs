use super::{AddressLookup, Address};
use super::super::{utxo::{UTxO}};
use cardano::address::ExtendedAddr;
use std::{fmt, error};

pub struct Accum();
impl Default for Accum { fn default() -> Self { Accum() } }

#[derive(Debug)]
pub struct Never;
impl fmt::Display for Never {
    fn fmt(&self, _: &mut fmt::Formatter) -> fmt::Result {
        unreachable!()
    }
}
impl error::Error for Never {}

impl AddressLookup for Accum {
    type Error = Never;

    fn lookup(&mut self, utxo: UTxO<ExtendedAddr>) -> Result<Option<UTxO<Address>>, Self::Error> {
        Ok(Some(utxo.map(|a| a.into())))
    }

    fn acknowledge<A: Into<Address>>(&mut self, _: A) -> Result<(), Self::Error> {
        Ok(())
    }
}
