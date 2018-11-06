use storage_units::{append, utils::{serialize, lock::{self, Lock}}};
use cardano::{block::{BlockDate, HeaderHash, types::EpochSlotId}};

use std::{
    error, fmt,
    fs::remove_file,
    io::{self, Read, Write},
    path::{Path, PathBuf},
    result,
};

use super::{ptr::{StatePtr}, utxo::{UTxO}};

use serde;
use serde_yaml;

#[derive(Debug)]
pub enum Error {
    LogNotFound,
    IoError(io::Error),
    LogFormatError(String),
    LockError(lock::Error),
    AppendError(append::Error),
    UnsupportedLogFormat(Vec<u8>)
}
impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self { Error::IoError(e) }
}
impl From<lock::Error> for Error {
    fn from(e: lock::Error) -> Self { Error::LockError(e) }
}
impl From<append::Error> for Error {
    fn from(e: append::Error) -> Self {
        match e {
            append::Error::NotFound => Error::LogNotFound,
            _ => Error::AppendError(e)
        }
    }
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::LogNotFound         => write!(f, "Log file not found"),
            Error::IoError(_)          => write!(f, "I/O Error"),
            Error::LogFormatError(err) => write!(f, "Log format error: `{}`", err),
            Error::LockError(_)        => write!(f, "Log's Lock file error"),
            Error::AppendError(_)      => write!(f, "Error when appending data to the log file"),
            Error::UnsupportedLogFormat(_) => {
                write!(f, "Unsupported Log format (tried to deserialize log of unknown encoding or log is corrupted)")
            }
        }
    }
}
impl error::Error for Error {
    fn cause(&self) -> Option<& error::Error> {
        match self {
            Error::LogNotFound             => None,
            Error::IoError(ref err)        => Some(err),
            Error::LogFormatError(_)       => None,
            Error::LockError(ref err)      => Some(err),
            Error::AppendError(ref err)    => Some(err),
            Error::UnsupportedLogFormat(_) => None,
        }
    }
}

pub type Result<T> = result::Result<T, Error>;

const MAGIC : &'static [u8] = b"EVT1";

#[derive(Debug, Serialize, Deserialize)]
pub enum Log<A> {
    Checkpoint(StatePtr),
    ReceivedFund(StatePtr, UTxO<A>),
    SpentFund(StatePtr, UTxO<A>)
}
impl<A: serde::Serialize> Log<A> {
    fn serialise(&self) -> Result<Vec<u8>> {
        let mut writer = Vec::with_capacity(64);

        let ptr = self.ptr();
        let date = ptr.latest_block_date();

        writer.write_all(b"EVT1")?;
        writer.write_all(ptr.latest_known_hash.as_ref())?;
        match date {
            BlockDate::Boundary(i) => {
                serialize::utils::write_u64(&mut writer, i as u64)?;
                serialize::utils::write_u64(&mut writer, u64::max_value())?;
            },
            BlockDate::Normal(i) => {
                serialize::utils::write_u64(&mut writer, i.epoch as u64)?;
                serialize::utils::write_u64(&mut writer, i.slotid as u64)?;
            },
        }

        match self {
            Log::Checkpoint(_) => {
                serialize::utils::write_u32(&mut writer, 1)?;
                serialize::utils::write_u64(&mut writer, 0)?;
            },
            Log::ReceivedFund(_, utxo) => {
                serialize::utils::write_u32(&mut writer, 2)?;
                serialize::utils::write_u64(&mut writer, 0)?;
                serde_yaml::to_writer(&mut writer, utxo).map_err(|e| {
                    Error::LogFormatError(format!("log format error: {:?}", e))
                })?;
            },
            Log::SpentFund(_, utxo) => {
                serialize::utils::write_u32(&mut writer, 3)?;
                serialize::utils::write_u64(&mut writer, 0)?;
                serde_yaml::to_writer(&mut writer, utxo).map_err(|e| {
                    Error::LogFormatError(format!("log format error: {:?}", e))
                })?;
            },
        }

        Ok(writer)
    }
}
impl<A> Log<A>
    where for<'de> A: serde::Deserialize<'de>
{
    fn deserisalise(bytes: &[u8]) -> Result<Self> {
        let mut reader = bytes;

        {
            let mut magic = [0u8; 4];
            reader.read_exact(&mut magic)?;
            if magic != MAGIC {
                return Err(Error::UnsupportedLogFormat(magic.iter().cloned().collect()));
            }
        }

        let ptr = {
            let mut hash = [0;32];
            reader.read_exact(&mut hash)?;
            let gen  = serialize::utils::read_u64(&mut reader)?;
            let slot = serialize::utils::read_u64(&mut reader)?;

            let hh = HeaderHash::from(hash);
            let bd = if slot == 0xFFFFFFFFFFFFFFFF {
                BlockDate::Boundary(gen as u64)
            } else {
                BlockDate::Normal(EpochSlotId { epoch: gen as u64, slotid: slot as u16 })
            };

            StatePtr::new(bd, hh)
        };

        let t = {
            let t = serialize::utils::read_u32(&mut reader)?;
            let b = serialize::utils::read_u64(&mut reader)?;
            debug_assert!(b == 0u64);
            t
        };

        match t {
            1 => Ok(Log::Checkpoint(ptr)),
            2 => {
                let utxo = serde_yaml::from_slice(reader).map_err(|e|
                    Error::LogFormatError(format!("log format error: {:?}", e))
                )?;
                Ok(Log::ReceivedFund(ptr, utxo))
            },
            3 => {
                let utxo = serde_yaml::from_slice(reader).map_err(|e|
                    Error::LogFormatError(format!("log format error: {:?}", e))
                )?;
                Ok(Log::SpentFund(ptr, utxo))
            },
            _ => {
                panic!("cannot parse log event of type: `{}'", t)
            }
        }
    }
}
impl<A> Log<A>
{
    pub fn ptr<'a>(&'a self) -> &'a StatePtr {
        match self {
            Log::Checkpoint(ptr) => ptr,
            Log::ReceivedFund(ptr, _) => ptr,
            Log::SpentFund(ptr, _) => ptr,
        }
    }
    pub fn map<F, U>(self, f: F) -> Log<U>
        where F: FnOnce(A) -> U
    {
        match self {
            Log::Checkpoint(ptr)    => Log::Checkpoint(ptr),
            Log::ReceivedFund(ptr, utxo) => Log::ReceivedFund(ptr, utxo.map(f)),
            Log::SpentFund(ptr, utxo)    => Log::SpentFund(ptr, utxo.map(f)),
        }
    }
}
impl<A: fmt::Display> fmt::Display for Log<A> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Log::Checkpoint(ptr)         => write!(f, "Checkpoint at: {}", ptr),
            Log::ReceivedFund(ptr, utxo) => write!(f, "Received funds at: {} {}", ptr, utxo),
            Log::SpentFund(ptr, utxo)    => write!(f, "Spent funds at: {} {}", ptr, utxo),
        }
    }
}

const WALLET_LOG_FILE : &'static str = "LOG";

pub struct LogLock {
    lock: lock::Lock,
    log_path: PathBuf,
}

impl LogLock {
    /// Acquires the lock on the log file of a given wallet.
    ///
    /// The lock will hold as long as the lifetime of the returned object.
    pub fn acquire<P: AsRef<Path>>(wallet_path: P) -> Result<Self> {
        let log_path = wallet_path.as_ref().join(WALLET_LOG_FILE);
        let lock = Lock::lock(log_path.clone())?;
        Ok(LogLock { lock, log_path })
    }

    /// Deletes the wallet log file, consuming the lock object.
    pub fn delete_wallet_log(self) -> Result<()> {
        Ok(remove_file(self.log_path)?)
    }
}

/// Structure to read the Wallet Log one by one
pub struct LogReader {
    inner: append::Reader,
    log_path: PathBuf,
}

impl LogReader {
    pub fn open(locked: LogLock) -> Result<Self> {
        let inner = append::Reader::open(locked.lock)?;
        Ok(LogReader {
            inner,
            log_path: locked.log_path,
        })
    }

    pub fn release_lock(self) -> LogLock {
        LogLock {
            lock: self.inner.close(),
            log_path: self.log_path,
        }
    }

    pub fn into_iter<A>(self) -> LogIterator<A>
        where for<'de> A: serde::Deserialize<'de>
    {
        LogIterator {reader: self, _log_type: ::std::marker::PhantomData }
    }
    pub fn next<A>(&mut self) -> Result<Option<Log<A>>>
        where for<'de> A: serde::Deserialize<'de>
    {
        match self.inner.next()? {
            None => Ok(None),
            Some(bytes) => {
                let log = Log::deserisalise(&bytes)?;
                Ok(Some(log))
            }
        }
    }
}

pub struct LogIterator<A> {
    reader: LogReader,
    _log_type: ::std::marker::PhantomData<A>
}
impl<A> Iterator for LogIterator<A>
    where for<'de> A: serde::Deserialize<'de>
{
    type Item = Result<Log<A>>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.reader.next() {
            Err(err) => Some(Err(err)),
            Ok(None) => None,
            Ok(Some(log)) => Some(Ok(log))
        }
    }
}

pub struct LogWriter {
    inner: append::Writer,
    log_path: PathBuf,
}

impl LogWriter {
    pub fn open(locked: LogLock) -> Result<Self> {
        let inner = append::Writer::open(locked.lock)?;
        Ok(LogWriter {
            inner,
            log_path: locked.log_path,
        })
    }

    pub fn release_lock(self) -> LogLock {
        LogLock {
            lock: self.inner.close(),
            log_path: self.log_path,
        }
    }

    pub fn append<A>(&mut self, log: &Log<A>) -> Result<()>
    where
        A: serde::Serialize + fmt::Debug,
    {
        let bytes = log.serialise()?;
        Ok(self.inner.append_bytes(&bytes)?)
    }
}
