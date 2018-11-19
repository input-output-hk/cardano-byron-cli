use storage_units::hash::BlockHash;

use std::{fmt, error};

#[derive(Debug)]
pub enum Error {
    IoError(::std::io::Error),
    CborError(::cbor_event::Error),
    StorageError(::cardano_storage::Error),
    InvalidBlockHash(BlockHash),
}
impl From<::std::io::Error> for Error {
    fn from(e: ::std::io::Error) -> Self { Error::IoError(e) }
}
impl From<::cbor_event::Error> for Error {
    fn from(e: ::cbor_event::Error) -> Self { Error::CborError(e) }
}
impl From<::cardano_storage::Error> for Error {
    fn from(e: ::cardano_storage::Error) -> Self { Error::StorageError(e) }
}

pub type Result<T> = ::std::result::Result<T, Error>;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::IoError(_)      => write!(f, "I/O Error"),
            Error::CborError(_)    => write!(f, "Encoding error (CBOR)"),
            Error::StorageError(_) => write!(f, "Storage error"),
            Error::InvalidBlockHash(_h) => {
                write!(f, "Invalid block hash")  // TODO: format the hash nicely
            }
        }
    }
}
impl error::Error for Error {
    fn cause(&self) -> Option<& error::Error> {
        match self {
            Error::IoError(ref err) => Some(err),
            Error::CborError(ref err) => Some(err),
            Error::StorageError(ref err) => Some(err),
            Error::InvalidBlockHash(_) => None,
        }
    }
}
