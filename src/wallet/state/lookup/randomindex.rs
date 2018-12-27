use cardano::hdwallet;
use cardano::{address::ExtendedAddr, hdwallet::XPrv};
use cardano::wallet::rindex;
use cardano::config::NetworkMagic;

use super::{Address, AddressLookup, AddressLookupError};
use super::super::{utxo::{UTxO}};

pub struct RandomIndexLookup {
    generator: rindex::AddressGenerator<hdwallet::XPrv>,
    network_magic: NetworkMagic,
}
impl RandomIndexLookup {
    pub fn from_wallet(wallet: rindex::Wallet, network_magic: NetworkMagic) -> Self {
        RandomIndexLookup {
            generator: wallet.address_generator(),
            network_magic: network_magic,
        }
    }
    pub fn new(generator: rindex::AddressGenerator<hdwallet::XPrv>, network_magic: NetworkMagic) -> Self {
        RandomIndexLookup {
            generator,
            network_magic
        }
    }

    pub fn get_private_key(&self, addr: &rindex::Addressing) -> XPrv {
        self.generator.key(addr)
    }

    pub fn get_address(&self, addr: &rindex::Addressing) -> ExtendedAddr {
        self.generator.address(addr, self.network_magic)
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
        use cardano::wallet::rindex;
        use cardano::hdpayload;
        let opt_addressing = match self.generator.try_get_addressing(&utxo.credited_address) {
            Ok(addressing) => addressing,
            Err(rindex::Error::PayloadError(hdpayload::Error::PayloadIsTooLarge(_))) => None,
            Err(rindex::Error::PayloadError(hdpayload::Error::NotEnoughEncryptedData)) => None,
            Err(err) => return Err(err.into()),
        };

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
