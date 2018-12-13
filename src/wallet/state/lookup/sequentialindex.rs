use cardano::wallet::{bip44};
use std::collections::BTreeMap;
use cardano::{address::{ExtendedAddr, Addr}, hdwallet::XPrv};
use cardano::config::NetworkMagic;

use super::{Address, AddressLookup, AddressLookupError};
use super::super::{utxo::{UTxO}};

pub const DEFAULT_GAP_LIMIT: u32 = 20;

type Result<T> = std::result::Result<T, AddressLookupError>;

pub struct SequentialBip44Lookup {
    // cryptographic wallet
    //
    // downside of needed the bip44's wallet is that we need to decrypt the
    // wallet private key with the password. This is needed because we might need
    // to create new addresses and they need hard derivation (which cannot be
    // done through the public key).
    //
    wallet: bip44::Wallet,
    // all the known expected addresses, that includes
    // all different accounts, and also the next not yet live
    // account's addresses
    expected: BTreeMap<Addr, bip44::Addressing>,

    // accounts threshold index for internal and external addresses
    accounts: Vec<[bip44::Index;2]>,

    network_magic: NetworkMagic,

    // gap limit
    gap_limit: u32,
}

impl SequentialBip44Lookup {
    pub fn new(wallet: bip44::Wallet, network_magic: NetworkMagic) -> Self {
        SequentialBip44Lookup {
            wallet: wallet,
            expected: BTreeMap::new(),
            accounts: Vec::new(),
            gap_limit: DEFAULT_GAP_LIMIT,
            network_magic: network_magic,
        }
    }

    pub fn get_private_key(&self, addr: &bip44::Addressing) -> bip44::IndexLevel<XPrv> {
        self.wallet.account(self.wallet.derivation_scheme(), addr.account.get_scheme_value())
                   .change(self.wallet.derivation_scheme(), addr.address_type())
                   .index(self.wallet.derivation_scheme(), addr.index.get_scheme_value())
    }

    pub fn get_address(&self, addr: &bip44::Addressing) -> ExtendedAddr {
        let xprv = self.get_private_key(addr);
        let xpub = xprv.public();
        ExtendedAddr::new_simple(*xpub, self.network_magic)
    }


    fn mut_generate_from(&mut self, account: &bip44::bip44::Account, change: u32, start: &bip44::Index, nb: u32) -> Result<()> {
        let max = start.incr(nb)?;
        let mut r = *start;
        // generate internal and external addresses
        while r < max {
            let addressing = bip44::Addressing { account: *account, change: change, index: r };
            let addr = self.get_address(&addressing);
            self.expected.insert(addr.into(), addressing);
            r = r.incr(1)?;
        }
        Ok(())
    }

    pub fn prepare_next_account(&mut self) -> Result<()> {
        // generate gap limit number of internal and external addresses in the account
        let account_nb = self.accounts.len() as u32;
        let account = bip44::bip44::Account::new(account_nb)?;
        let start = bip44::Index::new(0)?;
        let n = self.gap_limit;
        self.mut_generate_from(&account, 0, &start, n)?;
        self.mut_generate_from(&account, 1, &start, n)?;
        self.accounts.push([start, start]);
        Ok(())
    }

    // every time we find our address, we check if
    // the threshold for the next windows of address is met,
    // and if so, populate the expected cache with the new addresses and update the new threshold
    pub fn threshold_generate(&mut self, addressing: bip44::Addressing) -> Result<()> {
        if addressing.account.get_account_number() as usize >= self.accounts.len() {
            return Ok(());
        }
        let mut limits = self.accounts[addressing.account.get_account_number() as usize];
        if addressing.change != 0 && addressing.change != 1 {
            return Ok(());
        }
        let lidx = addressing.change as usize;
        let current_threshold = limits[lidx];
        if addressing.index <= current_threshold {
            return Ok(());
        }
        let new_threshold = current_threshold.incr(self.gap_limit)?;
        let gap = self.gap_limit;
        self.mut_generate_from(&addressing.account, addressing.change, &new_threshold, gap)?;
        limits[lidx] = new_threshold;
        Ok(())
    }
}

impl From<bip44::bip44::Error> for AddressLookupError {
    fn from(e: bip44::bip44::Error) -> Self {
        AddressLookupError::SequentialBip44(e)
    }
}

impl AddressLookup for SequentialBip44Lookup {
    fn lookup(
        &mut self,
        utxo: UTxO<ExtendedAddr>
    ) -> Result<Option<UTxO<Address>>> {
        let addressing = self.expected.get(&utxo.credited_address.to_address()).cloned();
        if let Some(addressing) = addressing {
            self.threshold_generate(addressing)?;

            Ok(Some(utxo.map(|_| addressing.into())))
        } else { Ok(None) }
    }

    fn acknowledge<A: Into<Address>>(&mut self, address: A) -> Result<()> {
        match address.into() {
            Address::Bip44(address) => self.threshold_generate(address),
            addr => {
                error!("unsupported address (expected bip44 addressing) {:#?}", addr);
                Err(bip44::bip44::Error::InvalidType(0).into())
            }
        }
    }
}
