use cardano::hdwallet;
use storage_units::utils::lock;
use std::{error, fmt};

use super::state::log;

/// wallet errors
#[derive(Debug)]
pub enum Error {
    CannotRetrievePrivateKeyInvalidPassword,
    CannotRetrievePrivateKey(hdwallet::Error),
    WalletLogAlreadyLocked(u32),
    WalletLogNotFound,
    WalletLogError(log::Error)
}
impl From<hdwallet::Error> for Error {
    fn from(e: hdwallet::Error) -> Self { Error::CannotRetrievePrivateKey(e) }
}
impl From<log::Error> for Error {
    fn from(e: log::Error) -> Self {
        match e {
            log::Error::LogNotFound => Error::WalletLogNotFound,
            log::Error::LockError(lock::Error::AlreadyLocked(_, process_id)) => Error::WalletLogAlreadyLocked(process_id),
            e => Error::WalletLogError(e)
        }
    }
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::CannotRetrievePrivateKeyInvalidPassword => write!(f, "Invalid spending password"),
            Error::CannotRetrievePrivateKey(_)             => write!(f, "Unsupported private key serialisation"),
            Error::WalletLogAlreadyLocked(pid)             => write!(f, "Wallet is already being used by another process (process id: {})", pid),
            Error::WalletLogNotFound                       => write!(f, "No wallet log Found"),
            Error::WalletLogError(_)                       => write!(f, "Error with the wallet log")
        }
    }
}
impl error::Error for Error {
    fn cause(&self) -> Option<& error::Error> {
        match self {
            Error::CannotRetrievePrivateKey(ref err)       => Some(err),
            Error::WalletLogError(ref err)                 => Some(err),
            Error::CannotRetrievePrivateKeyInvalidPassword => None,
            Error::WalletLogAlreadyLocked(_)               => None,
            Error::WalletLogNotFound                       => None,
        }
    }
}
