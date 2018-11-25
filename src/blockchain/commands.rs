use std::path::PathBuf;
use std::io::{Write};

use exe_common::config::net::Config;
use cardano_storage as storage;

use utils::{term::{Term, style::Style}, time};

use exe_common::parse_genesis_data;
use exe_common::genesis_data;
use super::{iter, peer, Blockchain, Result, Error, BlockchainName};
use cardano::{
    self,
    block::{BlockDate, HeaderHash, RawBlock},
};

/// function to create and initialize a given new blockchain
///
/// It will mainly create the subdirectories needed for the storage
/// of blocks, epochs and tags.
///
/// If the given blockchain configuration provides some preset peers
/// each peer will be initialized with an associated tag pointing to
/// the genesis hash of the blockchain (given in the same configuration
/// structure `Config`).
///
pub fn new( term: &mut Term
          , root_dir: PathBuf
          , name: BlockchainName
          , config: Config
          )
    -> Result<()>
{
    let blockchain = Blockchain::new(root_dir, name, config)?;
    blockchain.save();

    term.success(&format!("local blockchain `{}' created.\n", blockchain.name))?;

    Ok(())
}

pub fn list( term: &mut Term
           , root_dir: PathBuf
           , detailed: bool
           )
    -> Result<()>
{
    let blockchains_dir = super::config::blockchains_directory(&root_dir);
    let dir_reader = match ::std::fs::read_dir(blockchains_dir) {
        Err(err) => {
            use std::io::ErrorKind;
            return match err.kind() {
                ErrorKind::NotFound => Err(Error::ListNoBlockchains),
                ErrorKind::PermissionDenied => Err(Error::ListPermissionsDenied),
                _  => Err(Error::IoError(err)),
            }
        },
        Ok(dr) => dr
    };
    for entry in dir_reader {
        let entry = entry.unwrap();
        if ! entry.file_type()?.is_dir() {
            term.warn(&format!("unexpected file in blockchains directory: {:?}", entry.path()))?;
            continue;
        }
        let name = BlockchainName::from_os_str(entry.file_name())
            .map_err(Error::ListBlockchainInvalidName)?;

        let blockchain = Blockchain::load(root_dir.clone(), name)?;

        term.info(&blockchain.name)?;
        if detailed {
            let (tip, _is_genesis) = blockchain.load_tip();
            let tag_path = blockchain.dir.join("tag").join(super::LOCAL_BLOCKCHAIN_TIP_TAG);
            let metadata = ::std::fs::metadata(tag_path)?;
            let fetched_date = metadata.modified()?.into();
            let fetched_since = time::Duration::since(fetched_date);

            term.simply("\t")?;
            term.success(&format!("{} ({})", tip.hash, tip.date))?;
            term.simply("\t")?;
            term.warn(&format!("(updated {} ago)", style!(fetched_since)))?;
        }
        term.simply("\n")?;
    }

    Ok(())
}

pub fn destroy( term: &mut Term
              , root_dir: PathBuf
              , name: BlockchainName
              )
    -> Result<()>
{
    let blockchain = Blockchain::load(root_dir, name)?;

    writeln!(term, "You are about to destroy the local blockchain {}.
This means that all the blocks downloaded will be deleted and that the attached
wallets won't be able to interact with this blockchain.",
        ::console::style(&blockchain.name).bold().red(),
    )?;

    let confirmation = ::dialoguer::Confirmation::new().with_text("Are you sure?")
        .default(false)
        .interact()?;
    if confirmation {
        unsafe { blockchain.destroy() }?;
        term.success("blockchain successfully destroyed\n")?;
    }

    Ok(())
}

/// function to add a remote to the given blockchain
///
/// It will create the appropriate tag referring to the blockchain
/// genesis hash. This is because when add a new peer we don't assume
/// anything more than the genesis block.
///
pub fn remote_add( term: &mut Term
                 , root_dir: PathBuf
                 , name: BlockchainName
                 , remote_alias: String
                 , remote_endpoint: String
                 )
    -> Result<()>
{
    let mut blockchain = Blockchain::load(root_dir, name)?;
    blockchain.add_peer(remote_alias.clone(), remote_endpoint);
    blockchain.save();

    term.success(&format!("remote `{}' node added to blockchain `{}'\n", remote_alias, blockchain.name))?;

    Ok(())
}

/// remove the given peer from the blockchain
///
/// it will also delete all the metadata associated to this peer
/// such as the tag pointing to the remote's tip.
///
pub fn remote_rm( term: &mut Term
                , root_dir: PathBuf
                , name: BlockchainName
                , remote_alias: String
                )
    -> Result<()>
{
    let mut blockchain = Blockchain::load(root_dir, name)?;
    blockchain.remove_peer(remote_alias.clone());
    blockchain.save();

    term.success(&format!("remote `{}' node removed from blockchain `{}'\n", remote_alias, blockchain.name))?;

    Ok(())
}

pub fn remote_fetch( term: &mut Term
                   , root_dir: PathBuf
                   , name: BlockchainName
                   , peers: Vec<String>
                   )
    -> Result<()>
{
    let blockchain = Blockchain::load(root_dir, name)?;

    for np in blockchain.peers() {
        if peers.is_empty() || peers.contains(&np.name().to_owned()) {
            term.info(&format!("fetching blocks from peer: {}\n", np.name()))?;

            let peer = peer::Peer::prepare(&blockchain, np.name().to_owned());

            peer.connect(term).unwrap().sync(term);
        }
    }

    Ok(())
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub enum RemoteDetail {
    Short,
    Local,
    Remote
}

pub fn remote_ls( term: &mut Term
                , root_dir: PathBuf
                , name: BlockchainName
                , detailed: RemoteDetail
                )
    -> Result<()>
{
    let blockchain = Blockchain::load(root_dir, name)?;

    for np in blockchain.peers() {
        let peer = peer::Peer::prepare(&blockchain, np.name().to_owned());
        let (tip, _is_genesis) = peer.load_local_tip();

        writeln!(term, "{} ({})", style!(&peer.name), style!(&peer.config))?;

        if detailed >= RemoteDetail::Local {
            let tag_path = blockchain.dir.join("tag").join(&peer.tag);
            let metadata = ::std::fs::metadata(tag_path).unwrap();
            let fetched_date = metadata.modified()?.into();
            // get the difference between now and the last fetch, only keep up to the seconds
            let fetched_since = time::Duration::since(fetched_date);

            writeln!(term, " * last fetch:      {} ({} ago)", style!(fetched_date), style!(fetched_since))?;
            writeln!(term, " * local tip hash:  {}", style!(tip.hash))?;
            writeln!(term, " * local tip date:  {}", style!(tip.date))?;

            if detailed >= RemoteDetail::Remote {
                let mut connected_peer = peer.connect(term).unwrap();
                let remote_tip = connected_peer.query_tip();
                let block_diff = remote_tip.date - tip.date;

                writeln!(term, " * remote tip hash: {}", style!(remote_tip.hash))?;
                writeln!(term, " * remote tip date: {}", style!(remote_tip.date))?;
                writeln!(term, " * local is {} behind remote", style!(block_diff).red())?;
            }
        }
    }

    Ok(())
}

pub fn log( term: &mut Term
          , root_dir: PathBuf
          , name: BlockchainName
          , from: Option<HeaderHash>
          )
    -> Result<()>
{
    let blockchain = Blockchain::load(root_dir, name)?;

    let from = if let Some(hash) = from {
        if storage::block_location(&blockchain.storage, &hash).is_none() {
            return Err(Error::GetInvalidBlock(hash));
        }

        hash
    } else {
        blockchain.load_tip().0.hash
    };

    for block in storage::iter::ReverseIter::from(&blockchain.storage, from)? {
        use utils::pretty::Pretty;

        block.pretty(term, 0)?;
    }

    Ok(())
}

pub fn forward( term: &mut Term
              , root_dir: PathBuf
              , name: BlockchainName
              , to: Option<HeaderHash>
              )
    -> Result<()>
{
    let blockchain = Blockchain::load(root_dir, name)?;

    let hash = if let Some(hash) = to {
        if storage::block_location(&blockchain.storage, &hash).is_none() {
            return Err(Error::ForwardHashDoesNotExist(hash))
        }

        hash
    } else {
        let initial_tip = blockchain.load_tip().0;

        let tip = blockchain.peers().map(|np| {
            peer::Peer::prepare(&blockchain, np.name().to_owned()).load_local_tip().0
        }).fold(initial_tip, |current_tip, tip| {
            if tip.date > current_tip.date {
                tip
            } else {
                current_tip
            }
        });

        tip.hash
    };

    term.success(&format!("forward local tip to: {}\n", hash))?;

    blockchain.save_tip(&hash);

    Ok(())
}

pub fn pull( term: &mut Term
           , root_dir: PathBuf
           , name: BlockchainName
           )
    -> Result<()>
{
    let blockchain = Blockchain::load(root_dir.clone(), name.clone())?;

    for np in blockchain.peers() {
        if ! np.is_native() { continue; }
        term.info(&format!("fetching blocks from peer: {}\n", np.name()))?;

        let peer = peer::Peer::prepare(&blockchain, np.name().to_owned());

        peer.connect(term).unwrap().sync(term);
    }

    forward(term, root_dir, name, None)
}

fn get_block(blockchain: &Blockchain, hash: &HeaderHash) -> Result<RawBlock>
{
    let block_location = match storage::block_location(&blockchain.storage, &hash) {
        None => {
            return Err(Error::GetBlockDoesNotExist(hash.clone()));
        },
        Some(loc) => loc
    };

    debug!("blk location: {:?}", block_location);

    match storage::block_read_location(&blockchain.storage, &block_location, &hash) {
        None        => {
            // this is a bug, we have a block location available for this hash
            // but we were not able to read the block.
            return Err(Error::GetInvalidBlock(hash.clone()));
        },
        Some(rblk) => Ok(rblk)
    }
}

pub fn cat( term: &mut Term
          , root_dir: PathBuf
          , name: BlockchainName
          , hash: HeaderHash
          , no_parse: bool
          , debug: bool
          )
    -> Result<()>
{
    let blockchain = Blockchain::load(root_dir.clone(), name.clone())?;
    let rblk = get_block(&blockchain, &hash)?;

    if no_parse {
        ::std::io::stdout().write(rblk.as_ref())?;
        ::std::io::stdout().flush()?;
    } else {
        use utils::pretty::Pretty;

        let blk = rblk.decode().map_err(Error::CatMalformedBlock)?;
        if debug {
            writeln!(term, "{:#?}", blk)?;
        } else {
            blk.pretty(term, 0)?;
        }
    }

    Ok(())
}

pub fn status( term: &mut Term
             , root_dir: PathBuf
             , name: BlockchainName
             )
    -> Result<()>
{
    let blockchain = Blockchain::load(root_dir, name)?;

    writeln!(term, "{}", style!("Blockchain").cyan().bold())?;
    {
        let (tip, _is_genesis) = blockchain.load_tip();
        let tag_path = blockchain.dir.join("tag").join(super::LOCAL_BLOCKCHAIN_TIP_TAG);
        let metadata = ::std::fs::metadata(tag_path)?;
        let fetched_date = metadata.modified()?.into();
        // get the difference between now and the last fetch, only keep up to the seconds
        let fetched_since = time::Duration::since(fetched_date);

        writeln!(term, " * last forward:    {} ({} ago)", style!(fetched_date).green(), style!(fetched_since).green())?;
        writeln!(term, " * local tip hash:  {}", style!(tip.hash))?;
        writeln!(term, " * local tip date:  {}", style!(tip.date))?;
    }

    writeln!(term, "{}:", style!("Peers").cyan().bold())?;
    for (idx, np) in blockchain.peers().enumerate() {
        let peer = peer::Peer::prepare(&blockchain, np.name().to_owned());
        let (tip, _is_genesis) = peer.load_local_tip();

        writeln!(term, "  {}. {} ({})", style!(idx+1), style!(peer.name).cyan(), style!(peer.config).red())?;

        let tag_path = blockchain.dir.join("tag").join(&peer.tag);
        let metadata = ::std::fs::metadata(tag_path)?;
        let fetched_date = metadata.modified()?.into();
        // get the difference between now and the last fetch, only keep up to the seconds
        let fetched_since = time::Duration::since(fetched_date);

        writeln!(term, "   * last fetch:      {} ({} ago)", style!(fetched_date).green(), style!(fetched_since).green())?;
        writeln!(term, "   * local tip hash:  {}", style!(tip.hash))?;
        writeln!(term, "   * local tip date:  {}", style!(tip.date))?;
    }

    Ok(())
}

pub fn verify_block( term: &mut Term
                   , root_dir: PathBuf
                   , name: BlockchainName
                   , hash: HeaderHash
                   )
    -> Result<()>
{
    let blockchain = Blockchain::load(root_dir, name)?;
    let rblk = get_block(&blockchain, &hash)?;
    match rblk.decode() {
        Ok(blk) => {
            match cardano::block::verify_block(blockchain.config.protocol_magic, &hash, &blk) {
                Ok(()) => {
                    Ok(writeln!(term, "{}", style!("Block is valid").green())?)
                }
                Err(err) => {
                    Err(Error::VerifyInvalidBlock(err))
                }
            }
        },
        Err(err) => {
            Err(Error::VerifyMalformedBlock(err))
        }
    }
}

pub fn verify_chain( term: &mut Term
                   , root_dir: PathBuf
                   , name: BlockchainName
                   , stop_on_error: bool
                   )
    -> Result<()>
{
    let blockchain = Blockchain::load(root_dir, name)?;

    let tip = blockchain.load_tip().0;
    let num_blocks = tip.date.slot_number();

    let progress = term.progress_bar(num_blocks as u64);
    progress.set_message("verifying blocks... ");

    let genesis_data = {
        let genesis_data = genesis_data::get_genesis_data(&blockchain.config.genesis_prev)
            .map_err(Error::VerifyChainGenesisHashNotFound)?;

        parse_genesis_data::parse_genesis_data(genesis_data.as_bytes())
    };

    if genesis_data.genesis_prev != blockchain.config.genesis_prev {
        return Err(Error::VerifyChainInvalidGenesisPrevHash(blockchain.config.genesis_prev, genesis_data.genesis_prev));
    }

    let mut bad_blocks = 0;
    let mut chain_state = cardano::block::ChainState::new(&genesis_data);

    for res in blockchain.iter_to_tip(blockchain.config.genesis.clone())? {
        let (_raw_blk, blk) = res.unwrap();
        let hash = blk.get_header().compute_hash();
        match chain_state.verify_block(&hash, &blk) {
            Ok(()) => {},
            Err(err) => {
                bad_blocks += 1;
                writeln!(term, "Block {} ({}) is invalid", hash, blk.get_header().get_blockdate())?;
                writeln!(term, "    {:#?}", err)?;
                writeln!(term, "")?;
                if stop_on_error { break; }
            }
        }
        progress.inc(1);
    }

    progress.finish();

    writeln!(term, "verification finished:")?;
    writeln!(term, " * {} total blocks", style!(chain_state.chain_length).green().bold())?;
    writeln!(term, " * {} transactions", style!(chain_state.nr_transactions).green())?;
    writeln!(term, " * {} spent outputs", style!(chain_state.spent_txos).cyan())?;
    writeln!(term, " * {} unspent outputs", style!(chain_state.utxos.len()).cyan().bold())?;

    if bad_blocks > 0 {
        Err(Error::BlockchainIsNotValid(bad_blocks))
    } else {
        Ok(writeln!(term, "{}", style!("Blockchain is in a valid state").green())?)
    }
}

pub struct QueryParams {
    pub start: Option<BlockDate>,
    pub end: Option<BlockDate>,
}

pub fn query(
    term: &mut Term,
    root_dir: PathBuf,
    name: BlockchainName,
    params: QueryParams,
) -> Result<()> {
    let blockchain = Blockchain::load(root_dir, name)?;
    // FIXME: make blockchain.load_tip() return errors gracefully
    let tip = blockchain.load_tip().0.hash;
    let from = match params.start {
        Some(date) => {
            let resolved = storage::resolve_date_to_blockhash(
                &blockchain.storage,
                &tip,
                &date
            )?;
            match resolved {
                Some(hash) => hash,
                None => {
                    return Err(Error::QueryBlockDateNotResolved(date));
                }
            }
        }
        None => (*blockchain.config.genesis).clone(),
    };
    let to = match params.end {
        Some(date) => {
            let resolved = storage::resolve_date_to_blockhash(
                &blockchain.storage,
                &tip,
                &date
            )?;
            match resolved {
                Some(hash) => hash,
                None => {
                    return Err(Error::QueryBlockDateNotResolved(date));
                }
            }
        }
        None => (*tip).clone(),
    };
    for res in iter::Iter::new(&blockchain.storage, from, to)? {
        let (_raw_blk, block) = res?;
        let hash = block.get_header().compute_hash();
        writeln!(term, "{}", style!(hash));
    }
    Ok(())
}
