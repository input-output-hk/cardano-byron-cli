mod error;
pub mod epoch;

pub use self::error::{Error, Result};

use cardano::block::{Block, RawBlock, HeaderHash};
use cardano_storage::{self as storage, Storage};

enum IteratorType<'a> {
    Epoch(epoch::Epochs<'a>, Option<epoch::Iter>),
    Loose(&'a Storage, storage::iter::Range)
}
impl<'a> IteratorType<'a> {
    fn is_loose(&self) -> bool {
        match self {
            IteratorType::Loose(_, _) => true,
            _                         => false
        }
    }
}
impl<'a> Iterator for IteratorType<'a> {
    type Item = Result<RawBlock>;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            IteratorType::Epoch(ref mut epochs, ref mut opt_iter) => {
                if opt_iter.is_none() {
                    *opt_iter = match epochs.next() {
                        None           => None,
                        Some(Ok(v))    => Some(v),
                        Some(Err(err)) => { return Some(Err(err)); },
                    };
                }

                let res = if let Some(ref mut iter) = opt_iter {
                    iter.next()
                } else {
                    None
                };

                match res {
                    None => {
                        *opt_iter = match epochs.next() {
                            None           => None,
                            Some(Ok(v))    => Some(v),
                            Some(Err(err)) => { return Some(Err(err)); },
                        };
                        if let Some(ref mut iter) = opt_iter {
                            iter.next()
                        } else {
                            None
                        }
                    },
                    Some(res) => { Some(res) }
                }
            },
            IteratorType::Loose(ref storage, ref mut range) => {
                if let Some(bh) = range.next() {
                    let location = storage::BlockLocation::Loose;
                    Some(Ok(storage::block_read_location(&storage, &location, &bh)?))
                } else {
                    None
                }
            }
        }
    }
}

pub struct Iter<'a> {
    storage: &'a Storage,

    initialised: bool,

    starting_from: HeaderHash,
    ending_at: HeaderHash,

    last_known_block_hash: Option<HeaderHash>,

    iterator: IteratorType<'a>
}
impl<'a> Iter<'a> {
    pub fn new(storage: &'a Storage, from: HeaderHash, to: HeaderHash) -> Result<Self> {
        let iterator = match storage::block_location(&storage, &from) {
            None => {
                return Err(Error::InvalidBlockHash(from));
            }
            Some(storage::BlockLocation::Loose) => {
                let mut range = storage::iter::Range::new(
                    storage,
                    *from.clone(),
                    *to.clone()
                ).unwrap(); // TODO
                IteratorType::Loose(storage, range)
            },
            Some(location) => {
                let block_header = storage::block_read_location(&storage, &location, &from).unwrap().decode()?.get_header();
                let block_date = block_header.get_blockdate();

                let epochs = epoch::Epochs::new(&storage.config).from_epoch(block_date.get_epochid());
                let mut iterator = IteratorType::Epoch(epochs, None);

                iterator
            }
        };

        let iter = Iter {
            storage: storage,
            initialised: false,
            starting_from: from,
            ending_at: to,
            last_known_block_hash: None,
            iterator: iterator
        };

        Ok(iter)
    }
}
impl<'a> Iterator for Iter<'a> {
    type Item = Result<(RawBlock, Block)>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(ref hh) = self.last_known_block_hash {
            if hh == &self.ending_at { return None; }
        }

        if ! self.initialised {
            self.initialised = true;

            let mut next = None;
            for block in self.iterator.next() {
                match block {
                    Err(err) => return Some(Err(err)),
                    Ok(raw_block) => {
                        let block = raw_block.decode().unwrap();
                        let hh = block.get_header().compute_hash();
                        let end = &hh == &self.starting_from;
                        next = Some(Ok((raw_block, block)));
                        self.last_known_block_hash = Some(hh);
                        if end { break; }
                    }
                }
            }

            next
        } else {
            match self.iterator.next() {
                None => {
                    if ! self.iterator.is_loose() {
                        let mut range = storage::iter::Range::new(
                            &self.storage,
                            *self.last_known_block_hash.clone().unwrap(),
                            *self.ending_at.clone()
                        ).unwrap(); // TODO
                        range.next(); // remove the last known block hash (it was the one in the last epoch)
                        self.iterator = IteratorType::Loose(&self.storage, range);
                        self.next()
                    } else {
                        None
                    }
                },
                Some(Err(err)) => Some(Err(err)),
                Some(Ok(raw_block)) => {
                    let block = raw_block.decode().unwrap();
                    let hh = block.get_header().compute_hash();
                    self.last_known_block_hash = Some(hh);
                    Some(Ok((raw_block, block)))
                }
            }
        }
    }
}
