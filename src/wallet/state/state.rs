use super::utxo::{UTxO, UTxOs};
use super::log::{Log};
use super::{lookup::{AddressLookup, Address}, ptr::StatePtr};
use cardano::{tx::TxoPointer, coin::{self, Coin}, address::ExtendedAddr};

#[derive(Debug)]
pub struct State<T: AddressLookup> {
    pub ptr: StatePtr,
    pub lookup_struct: T,
    pub utxos: UTxOs<Address>
}

impl<T: AddressLookup> State<T> {
    pub fn new(ptr: StatePtr, lookup_struct: T) -> Self {
        State { ptr: ptr, lookup_struct: lookup_struct, utxos: UTxOs::new() }
    }

    pub fn from_logs<I: IntoIterator<Item = Log<Address>>>(mut lookup_struct: T, iter: I) -> Result<Result<Self, T>, T::Error>
    {
        let mut ptr = None;
        let mut utxos = UTxOs::new();

        for log in iter {
            match log {
                Log::Checkpoint(known_ptr) => ptr = Some(known_ptr),
                Log::ReceivedFund(known_ptr, utxo) => {
                    lookup_struct.acknowledge(utxo.credited_addressing.clone())?;
                    ptr = Some(known_ptr);

                    if let Some(utxo) = utxos.insert(utxo.extract_txin(), utxo) {
                        error!("This UTxO was already in the UTxOs collection `{}'", utxo);
                        panic!("The Wallet LOG file seems corrupted");
                    };
                },
                Log::SpentFund(known_ptr, utxo) => {
                    match utxos.remove(&utxo.extract_txin()) {
                        Some(_) => { },
                        None    => {
                            error!("UTxO not in the known UTxOs collection `{}'", utxo);
                            panic!("The Wallet LOG file seems corrupted");
                        }
                    };
                    lookup_struct.acknowledge(utxo.credited_addressing.clone())?;
                    ptr = Some(known_ptr);
                },
            }
        }

        if let Some(ptr) = ptr {
           Ok(Ok(State { ptr: ptr, lookup_struct: lookup_struct, utxos: utxos }))
        } else {
            Ok(Err(lookup_struct))
        }
    }

    pub fn ptr<'a>(&'a self) -> &'a StatePtr { &self.ptr }

    pub fn total(&self) -> coin::Result<Coin> {
        self.utxos
            .iter()
            .map(|(_, v)| v.credited_value)
            .fold(Ok(Coin::zero()), |acc, v| {
                acc.and_then(|acc| acc + v)
            })
    }

    pub fn forward_with_txins<'a, I>(&mut self, iter: I) -> Result<Vec<Log<Address>>, T::Error>
        where I: IntoIterator<Item = (StatePtr, &'a TxoPointer)>
    {
        let mut events = Vec::new();
        for (ptr, txin) in iter {
            if let Some(utxo) = self.utxos.remove(&txin) {
                events.push(Log::SpentFund(ptr, utxo.clone()));
            }
        }
        Ok(events)
    }
    pub fn forward_with_utxos<I>(&mut self, iter: I) -> Result<Vec<Log<Address>>, T::Error>
        where I: IntoIterator<Item = (StatePtr, UTxO<ExtendedAddr>)>
    {
        let mut events = Vec::new();
        for (ptr, utxo) in iter {
            if let Some(utxo) = self.lookup_struct.lookup(utxo)? {
                self.ptr = ptr.clone();
                events.push(Log::ReceivedFund(ptr, utxo.clone()));
                self.utxos.insert(utxo.extract_txin(), utxo);
            }
        }
        Ok(events)
    }
}
