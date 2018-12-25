use super::{
    log::{self, Log},
    lookup::{Address, AddressLookup, AddressLookupError},
    ptr::StatePtr,
    utxo::{UTxO, UTxOs},
};
use cardano::{
    address::ExtendedAddr,
    coin::{self, Coin},
    tx::TxoPointer,
};

use std::{
    error::Error,
    fmt::{self, Debug, Display},
};

#[derive(Debug)]
pub struct State<T: AddressLookup> {
    pub ptr: StatePtr,
    pub lookup_struct: T,
    pub utxos: UTxOs<Address>,
}

/// Errors that may be returned by `State::from_log`.
#[derive(Debug)]
pub enum FromLogsError<T> {
    /// No entries retrieved. Returns back the lookup structure.
    NoEntries(T),
    /// Error reported by the LogReader.
    LogReadFailed(log::Error),
    /// Failed to look up an address from the log.
    AddressLookupFailed(AddressLookupError),
}

impl<T> From<log::Error> for FromLogsError<T> {
    fn from(e: log::Error) -> Self {
        FromLogsError::LogReadFailed(e)
    }
}

impl<T> From<AddressLookupError> for FromLogsError<T> {
    fn from(e: AddressLookupError) -> Self {
        FromLogsError::AddressLookupFailed(e)
    }
}

impl<T> Display for FromLogsError<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::FromLogsError::*;
        match self {
            NoEntries(_) => write!(f, "No entries in the log"),
            LogReadFailed(_) => write!(f, "Failed to read log"),
            AddressLookupFailed(_) => write!(f, "Failed to look up an address found in the log"),
        }
    }
}

impl<T: Debug> Error for FromLogsError<T> {
    fn cause(&self) -> Option<&Error> {
        use self::FromLogsError::*;
        match self {
            NoEntries(_) => None,
            LogReadFailed(err) => Some(err),
            AddressLookupFailed(err) => Some(err),
        }
    }
}

impl<T: AddressLookup> State<T> {
    pub fn new(ptr: StatePtr, lookup_struct: T) -> Self {
        State {
            ptr: ptr,
            lookup_struct: lookup_struct,
            utxos: UTxOs::new(),
        }
    }

    pub fn from_logs<I>(mut lookup_struct: T, iter: I) -> Result<Self, FromLogsError<T>>
    where
        I: IntoIterator<Item = Result<Log<Address>, log::Error>>,
    {
        let mut ptr = None;
        let mut utxos = UTxOs::new();

        for entry in iter {
            let log = entry?;
            match log {
                Log::Checkpoint(known_ptr) => ptr = Some(known_ptr),
                Log::ReceivedFund(known_ptr, utxo) => {
                    lookup_struct.acknowledge(utxo.credited_addressing.clone())?;
                    ptr = Some(known_ptr);

                    if let Some(utxo) = utxos.insert(utxo.extract_txin(), utxo) {
                        error!("This UTxO was already in the UTxOs collection `{}'", utxo);
                        panic!("The Wallet LOG file seems corrupted");
                    };
                }
                Log::SpentFund(known_ptr, utxo) => {
                    match utxos.remove(&utxo.extract_txin()) {
                        Some(_) => {}
                        None => {
                            error!("UTxO not in the known UTxOs collection `{}'", utxo);
                            panic!("The Wallet LOG file seems corrupted");
                        }
                    };
                    lookup_struct.acknowledge(utxo.credited_addressing.clone())?;
                    ptr = Some(known_ptr);
                }
            }
        }

        if let Some(ptr) = ptr {
            Ok(State {
                ptr: ptr,
                lookup_struct: lookup_struct,
                utxos: utxos,
            })
        } else {
            Err(FromLogsError::NoEntries(lookup_struct))
        }
    }

    pub fn ptr<'a>(&'a self) -> &'a StatePtr {
        &self.ptr
    }

    pub fn total(&self) -> coin::Result<Coin> {
        self.utxos
            .iter()
            .map(|(_, v)| v.credited_value)
            .fold(Ok(Coin::zero()), |acc, v| acc.and_then(|acc| acc + v))
    }

    pub fn forward_with_txins<'a, I>(
        &mut self,
        iter: I,
    ) -> Result<Vec<Log<Address>>, AddressLookupError>
    where
        I: IntoIterator<Item = (StatePtr, &'a TxoPointer)>,
    {
        let mut events = Vec::new();
        for (ptr, txin) in iter {
            if let Some(utxo) = self.utxos.remove(&txin) {
                events.push(Log::SpentFund(ptr, utxo.clone()));
            }
        }
        Ok(events)
    }

    pub fn forward_with_utxos<I>(
        &mut self,
        iter: I,
    ) -> Result<Vec<Log<Address>>, AddressLookupError>
    where
        I: IntoIterator<Item = (StatePtr, UTxO<ExtendedAddr>)>,
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
