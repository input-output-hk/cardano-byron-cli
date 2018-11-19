use cardano::hdwallet;
use cardano::{address::ExtendedAddr, hdwallet::XPrv};
use cardano::wallet::rindex;

use super::{Address, AddressLookup, AddressLookupError};
use super::super::{utxo::{UTxO}};

pub struct RandomIndexLookup {
    generator: rindex::AddressGenerator<hdwallet::XPrv>
}
impl From<rindex::Wallet> for RandomIndexLookup {
    fn from(wallet: rindex::Wallet) -> Self {
        RandomIndexLookup {
            generator: wallet.address_generator()
        }
    }
}
impl RandomIndexLookup {
    pub fn new(generator: rindex::AddressGenerator<hdwallet::XPrv>) -> Self {
        RandomIndexLookup {
            generator
        }
    }

    pub fn get_private_key(&self, addr: &rindex::Addressing) -> XPrv {
        self.generator.key(addr)
    }

    pub fn get_address(&self, addr: &rindex::Addressing) -> ExtendedAddr {
        self.generator.address(addr)
    }
}

impl From<rindex::Error> for AddressLookupError {
    fn from(e: rindex::Error) -> Self {
        AddressLookupError::RandomIndex(e)
    }
}

impl AddressLookup for RandomIndexLookup {
    /// Random index lookup is more a random index decryption and reconstruction method
    ///
    /// 1. we check if the input address contains a derivation_path (see cardano::address's ExtendedAddress);
    /// 2. we reconstruct the address with the derivation path and check it is actually one of ours;
    ///
    fn lookup(
        &mut self,
        utxo: UTxO<ExtendedAddr>,
    ) -> Result<Option<UTxO<Address>>, AddressLookupError> {
        let opt_addressing = self.generator.try_get_addressing(&utxo.credited_address)?;

        match opt_addressing {
            None => Ok(None),
            Some(addressing) => {
                let address = self.get_address(&addressing);

                if address != utxo.credited_address {
                    debug!("credited address:    {}", utxo.credited_address);
                    debug!("constructed address: {}", address);
                    Err(rindex::Error::CannotReconstructAddress(address).into())
                } else {
                    Ok(Some(utxo.map(|_| addressing.into())))
                }
            }
        }
    }

    /// in the case of random index lookup there is nothing to acknowledge
    /// the addresses are self descriptive and we don't need to keep metadata
    /// or state to update.
    ///
    /// This function does nothing and always succeeds
    fn acknowledge<A: Into<Address>>(
        &mut self,
        _: A,
    ) -> Result<(), AddressLookupError> {
        Ok(())
    }
}
