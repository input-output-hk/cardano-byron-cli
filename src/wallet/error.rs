use blockchain;
use cardano::{bip::bip44, coin, hdwallet, wallet::rindex};
use serde_yaml;
use storage_units::utils::lock;

use std::{error, fmt, io, path::PathBuf};

use super::state::log;

/// wallet errors
#[derive(Debug)]
pub enum Error {
    IoError(io::Error),
    CannotLoadBlockchain(blockchain::Error),
    CoinError(coin::Error),
    Bip44AddressError(bip44::Error),
    CannotRetrievePrivateKey(hdwallet::Error),
    CannotRetrievePrivateKeyInvalidPassword,
    CannotRecoverFromDaedalusMnemonics(rindex::Error),
    ConfigReadFailed(PathBuf, serde_yaml::Error),
    ConfigWriteFailed(PathBuf, serde_yaml::Error),
    WalletLoadFailed(io::Error),
    WalletSaveFailed(io::Error),
    WalletDestroyFailed(io::Error),
    WalletDeleteLogFailed(io::Error),
    WalletLogAlreadyLocked(u32),
    WalletLogNotFound,
    WalletLogError(log::Error),
    AttachAlreadyAttached(String),
}
impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self { Error::IoError(e) }
}
impl From<blockchain::Error> for Error {
    fn from(e: blockchain::Error) -> Self { Error::CannotLoadBlockchain(e) }
}
impl From<coin::Error> for Error {
    fn from(e: coin::Error) -> Self { Error::CoinError(e) }
}
impl From<bip44::Error> for Error {
    fn from(e: bip44::Error) -> Self { Error::Bip44AddressError(e) }
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
            Error::IoError(_)                              => write!(f, "I/O error occurred"),
            Error::CannotLoadBlockchain(_)                 => write!(f, "Cannot load blockchain"),
            Error::CoinError(_)                            => write!(f, "Error with coin calculations"),
            Error::Bip44AddressError(_)                    => write!(f, "Error with BIP44 account addressing"),
            Error::CannotRetrievePrivateKey(_)             => write!(f, "Unsupported private key serialisation"),
            Error::CannotRetrievePrivateKeyInvalidPassword => write!(f, "Invalid spending password"),
            Error::CannotRecoverFromDaedalusMnemonics(_)   => write!(f, "Cannot recover the wallet from Daedalus mnemonics"),
            Error::ConfigReadFailed(ref path, _)           => write!(f, "Failed to read wallet configuration file `{}`", path.to_string_lossy()),
            Error::ConfigWriteFailed(ref path, _)          => write!(f, "Failed to write wallet configuration to directory `{}`", path.to_string_lossy()),
            Error::WalletLoadFailed(_)                     => write!(f, "Cannot load the wallet"),
            Error::WalletSaveFailed(_)                     => write!(f, "Cannot save the wallet"),
            Error::WalletDestroyFailed(_)                  => write!(f, "Cannot destroy the wallet"),
            Error::WalletDeleteLogFailed(_)                => write!(f, "Cannot delete the wallet's log"),
            Error::WalletLogAlreadyLocked(pid)             => write!(f, "Wallet is already being used by another process (process id: {})", pid),
            Error::WalletLogNotFound                       => write!(f, "No wallet log Found"),
            Error::WalletLogError(_)                       => write!(f, "Error with the wallet log"),
            Error::AttachAlreadyAttached(bn)               => write!(f, "Wallet already attached to blockchain `{}'", bn),
        }
    }
}
impl error::Error for Error {
    fn cause(&self) -> Option<& error::Error> {
        match self {
            Error::IoError(ref err)                        => Some(err),
            Error::CannotLoadBlockchain(ref err)           => Some(err),
            Error::CoinError(ref err)                      => Some(err),
            Error::Bip44AddressError(ref err)              => Some(err),
            Error::CannotRetrievePrivateKey(ref err)       => Some(err),
            Error::CannotRetrievePrivateKeyInvalidPassword => None,
            Error::CannotRecoverFromDaedalusMnemonics(ref err) => Some(err),
            Error::ConfigReadFailed(_, ref err)            => Some(err),
            Error::ConfigWriteFailed(_, ref err)           => Some(err),
            Error::WalletLoadFailed(ref err)               => Some(err),
            Error::WalletSaveFailed(ref err)               => Some(err),
            Error::WalletDestroyFailed(ref err)            => Some(err),
            Error::WalletDeleteLogFailed(ref err)          => Some(err),
            Error::WalletLogAlreadyLocked(_)               => None,
            Error::WalletLogNotFound                       => None,
            Error::WalletLogError(ref err)                 => Some(err),
            Error::AttachAlreadyAttached(_)                => None,
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;
