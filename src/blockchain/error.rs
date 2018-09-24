use std::{io, fmt::{self, Display, Formatter}, error::{self}};
use cardano_storage;

#[derive(Debug)]
pub enum Error {
    IoError(io::Error),

    NewCannotInitializeBlockchainDirectory(cardano_storage::Error),

    ListNoBlockchains,
    ListPermissionsDenied,
    ListBlockchainWithNonUTF8Name(::std::ffi::OsString),
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self { Error::IoError(e) }
}

pub type Result<T> = ::std::result::Result<T, Error>;
