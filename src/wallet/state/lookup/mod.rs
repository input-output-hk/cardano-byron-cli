use super::utxo::UTxO;
use cardano::address::ExtendedAddr;

mod address;
mod error;

pub mod accum;
pub mod randomindex;
pub mod sequentialindex;

pub use self::address::Address;
pub use self::error::AddressLookupError;

pub trait AddressLookup {
    /// the implementor will attempt the recognize the given UTxO's credited_address.
    ///
    /// In the case of sequential address, it will be a lookup of the generated address against
    /// every known address plus the look ahead threshold.
    ///
    /// In the case of random address it will mainly be an attempt to decrypt the
    /// given hdpayload and reconstructing the address with it.
    ///
    fn lookup(
        &mut self,
        utxo: UTxO<ExtendedAddr>,
    ) -> Result<Option<UTxO<Address>>, AddressLookupError>;

    /// this function will allow the implementor to update its initial state.
    /// This is in the case of wallet using sequential indices for the addresses.
    ///
    /// When the wallet will load the wallet log, this will allow the address lookup
    /// object to update its state before the main operation starts.
    ///
    fn acknowledge<A>(&mut self, address: A) -> Result<(), AddressLookupError>
    where
        A: Into<Address>;
}
