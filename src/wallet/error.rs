use blockchain::{self, BlockchainName};
use cardano::{hdwallet, wallet::rindex};
use storage_units::utils::lock;
use std::{error, fmt, io};

use super::state::log;

/// wallet errors
#[derive(Debug)]
pub enum Error {
    CannotRetrievePrivateKeyInvalidPassword,
    CannotRetrievePrivateKey(hdwallet::Error),
    CannotRecoverFromDaedalusMnemonics(rindex::Error),
    WalletDestroyFailed(io::Error),
    WalletDeleteLogFailed(io::Error),
    WalletLogAlreadyLocked(u32),
    WalletLogNotFound,
    WalletLogError(log::Error),
    CannotLoadBlockchain(blockchain::Error),
    AttachAlreadyAttached(String),
}
impl From<blockchain::Error> for Error {
    fn from(e: blockchain::Error) -> Self { Error::CannotLoadBlockchain(e) }
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
            Error::CannotRecoverFromDaedalusMnemonics(_)   => write!(f, "Cannot recover the wallet from Daedalus mnemonics"),
            Error::WalletDestroyFailed(_)                  => write!(f, "Cannot destroy the wallet"),
            Error::WalletDeleteLogFailed(_)                => write!(f, "Cannot delete the wallet's log"),
            Error::WalletLogAlreadyLocked(pid)             => write!(f, "Wallet is already being used by another process (process id: {})", pid),
            Error::WalletLogNotFound                       => write!(f, "No wallet log Found"),
            Error::WalletLogError(_)                       => write!(f, "Error with the wallet log"),
            Error::CannotLoadBlockchain(_)                 => write!(f, "Cannot load blockchain"),
            Error::AttachAlreadyAttached(bn)               => write!(f, "Wallet already attached to blockchain `{}'", bn),
        }
    }
}
impl error::Error for Error {
    fn cause(&self) -> Option<& error::Error> {
        match self {
            Error::CannotRetrievePrivateKeyInvalidPassword => None,
            Error::CannotRetrievePrivateKey(ref err)       => Some(err),
            Error::CannotRecoverFromDaedalusMnemonics(ref err) => Some(err),
            Error::WalletDestroyFailed(ref err)            => Some(err),
            Error::WalletDeleteLogFailed(ref err)          => Some(err),
            Error::WalletLogAlreadyLocked(_)               => None,
            Error::WalletLogNotFound                       => None,
            Error::WalletLogError(ref err)                 => Some(err),
            Error::CannotLoadBlockchain(ref err)           => Some(err),
            Error::AttachAlreadyAttached(_)                => None,
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;
