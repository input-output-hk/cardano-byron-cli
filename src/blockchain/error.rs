use std::{io, fmt, error, path::PathBuf};
use cardano::block::{self, BlockDate, HeaderHash};
use cardano_storage;
use cbor_event;

#[derive(Debug)]
pub enum Error {
    IoError(io::Error),
    StorageError(cardano_storage::Error),

    NewCannotInitializeBlockchainDirectory(cardano_storage::Error),

    LoadConfigFileNotFound(PathBuf),

    ListNoBlockchains,
    ListPermissionsDenied,
    ListBlockchainInvalidName(::storage_units::utils::directory_name::DirectoryNameError),

    ForwardHashDoesNotExist(HeaderHash),

    GetBlockDoesNotExist(HeaderHash),
    GetInvalidBlock(HeaderHash),

    CatMalformedBlock(cbor_event::Error),

    VerifyInvalidBlock(block::Error),
    VerifyMalformedBlock(cbor_event::Error),

    VerifyChainGenesisHashNotFound(HeaderHash),
    VerifyChainInvalidGenesisPrevHash(HeaderHash, HeaderHash), // (Expected, got)
    BlockchainIsNotValid(usize),

    QueryBlockDateNotResolved(BlockDate),
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self { Error::IoError(e) }
}

impl From<cardano_storage::Error> for Error {
    fn from(e: cardano_storage::Error) -> Self { Error::StorageError(e) }
}

pub type Result<T> = ::std::result::Result<T, Error>;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::IoError(_) => write!(f, "I/O Error"),
            Error::StorageError(_) => write!(f, "Storage Error"),

            Error::NewCannotInitializeBlockchainDirectory(_) => write!(f, "Cannot Initialise the blockchain directory"),
            Error::LoadConfigFileNotFound(p)                 => write!(f, "Cannot load blockchain configuration from `{}`; is the blockchain initialized?", p.to_string_lossy()),
            Error::ListNoBlockchains                         => write!(f, "No local blockchains yet"),
            Error::ListPermissionsDenied                     => write!(f, "No local blockchains (permission denied to the cardano-cli directory, check the `root-dir` option of the CLI)"),
            Error::ListBlockchainInvalidName(_)              => write!(f, "Blockchain with invalid name"),
            Error::ForwardHashDoesNotExist(hh)               => write!(f, "Cannot forward the blockchain to non existant hash `{}`", hh),
            Error::GetBlockDoesNotExist(hh)                  => write!(f, "Block `{}` does not exist", hh),
            Error::GetInvalidBlock(hh)                       => write!(f, "Block `{}` cannot be read from the local storage", hh),
            Error::CatMalformedBlock(_)                      => write!(f, "Unsupported or corrupted block"),
            Error::VerifyInvalidBlock(_)                     => write!(f, "Block is not valid"),
            Error::VerifyMalformedBlock(_)                   => write!(f, "Unsupported or corrupted block"),
            Error::VerifyChainGenesisHashNotFound(hh)        => write!(f, "Genesis data for given blockchain not found ({})", hh),
            Error::VerifyChainInvalidGenesisPrevHash(eh, hh) => write!(f, "Genesis data invalid: expected previous hash {} different from the one provided {}", eh, hh),
            Error::BlockchainIsNotValid(num_invalid_blocks)  => write!(f, "Blockchain has {} invalid blocks", num_invalid_blocks),
            Error::QueryBlockDateNotResolved(date) => {
                write!(f, "Cannot resolve block date {}", date)
            }
        }
    }
}
impl error::Error for Error {
    fn cause(&self) -> Option<& error::Error> {
        match self {
            Error::IoError(ref err) => Some(err),
            Error::StorageError(ref err) => Some(err),
            Error::NewCannotInitializeBlockchainDirectory(ref err) => Some(err),
            Error::ListBlockchainInvalidName(ref err) => Some(err),
            Error::CatMalformedBlock(ref err) => Some(err),
            Error::VerifyInvalidBlock(ref err) => Some(err),
            Error::VerifyMalformedBlock(ref err) => Some(err),
            _ => None
        }
    }
}
