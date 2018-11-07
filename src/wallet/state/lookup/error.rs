use cardano::wallet::{bip44, rindex};

use std::{
    error::Error,
    fmt::{self, Display},
};

#[derive(Debug)]
pub enum AddressLookupError {
    RandomIndex(rindex::Error),
    SequentialBip44(bip44::bip44::Error),
}

impl Display for AddressLookupError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::AddressLookupError::*;

        match self {
            RandomIndex(e) => e.fmt(f),
            SequentialBip44(e) => e.fmt(f),
        }
    }
}

impl Error for AddressLookupError {}
