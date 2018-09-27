use std::path::PathBuf;
use std::io::{Write};

use exe_common::config::net::Config;
use cardano_storage as storage;

use utils::{term::{Term, style::Style}, time};

use super::parse_genesis_data;
use super::genesis_data;
use super::{peer, Blockchain, Result, Error, BlockchainName};
use cardano::{self, block::{RawBlock, HeaderHash}};

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
pub fn new( mut term: Term
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

pub fn list( mut term: Term
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

        let blockchain = Blockchain::load(root_dir.clone(), name);

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

pub fn destroy( mut term: Term
              , root_dir: PathBuf
              , name: BlockchainName
              )
{
    let blockchain = Blockchain::load(root_dir, name);

    writeln!(term, "You are about to destroy the local blockchain {}.
This means that all the blocks downloaded will be deleted and that the attached
wallets won't be able to interact with this blockchain.",
        ::console::style(&blockchain.name).bold().red(),
    ).unwrap();

    let confirmation = ::dialoguer::Confirmation::new("Are you sure?")
        .use_line_input(true)
        .clear(false)
        .default(false)
        .interact().unwrap();
    if ! confirmation { ::std::process::exit(0); }

    unsafe { blockchain.destroy() }.unwrap();

    term.success("blockchain successfully destroyed\n").unwrap();
}

/// function to add a remote to the given blockchain
///
/// It will create the appropriate tag referring to the blockchain
/// genesis hash. This is because when add a new peer we don't assume
/// anything more than the genesis block.
///
pub fn remote_add( mut term: Term
                 , root_dir: PathBuf
                 , name: BlockchainName
                 , remote_alias: String
                 , remote_endpoint: String
                 )
{
    let mut blockchain = Blockchain::load(root_dir, name);
    blockchain.add_peer(remote_alias.clone(), remote_endpoint);
    blockchain.save();

    term.success(&format!("remote `{}' node added to blockchain `{}'\n", remote_alias, blockchain.name)).unwrap();
}

/// remove the given peer from the blockchain
///
/// it will also delete all the metadata associated to this peer
/// such as the tag pointing to the remote's tip.
///
pub fn remote_rm( mut term: Term
                , root_dir: PathBuf
                , name: BlockchainName
                , remote_alias: String
                )
{
    let mut blockchain = Blockchain::load(root_dir, name);
    blockchain.remove_peer(remote_alias.clone());
    blockchain.save();

    term.success(&format!("remote `{}' node removed from blockchain `{}'\n", remote_alias, blockchain.name)).unwrap();
}

pub fn remote_fetch( mut term: Term
                   , root_dir: PathBuf
                   , name: BlockchainName
                   , peers: Vec<String>
                   )
{
    let blockchain = Blockchain::load(root_dir, name);

    for np in blockchain.peers() {
        if peers.is_empty() || peers.contains(&np.name().to_owned()) {
            term.info(&format!("fetching blocks from peer: {}\n", np.name())).unwrap();

            let peer = peer::Peer::prepare(&blockchain, np.name().to_owned());

            peer.connect(&mut term).unwrap().sync(&mut term);
        }
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub enum RemoteDetail {
    Short,
    Local,
    Remote
}

pub fn remote_ls( mut term: Term
                , root_dir: PathBuf
                , name: BlockchainName
                , detailed: RemoteDetail
                )
{
    let blockchain = Blockchain::load(root_dir, name);

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
    let blockchain = Blockchain::load(root_dir, name);

    let from = if let Some(hash) = from {
        if storage::block_location(&blockchain.storage, &hash).is_none() {
            term.error(&format!("block hash `{}' is not present in the local blockchain\n", hash))?;
            ::std::process::exit(1);
        }

        hash
    } else {
        blockchain.load_tip().0.hash
    };

    for block in storage::block::iter::ReverseIter::from(&blockchain.storage, from).unwrap() {
        use utils::pretty::Pretty;

        block.pretty(&mut term, 0).unwrap();
    }
}

pub fn forward( mut term: Term
              , root_dir: PathBuf
              , name: BlockchainName
              , to: Option<HeaderHash>
              )
{
    let blockchain = Blockchain::load(root_dir, name);

    let hash = if let Some(hash) = to {
        if storage::block_location(&blockchain.storage, &hash).is_none() {
            term.error(&format!("block hash `{}' is not present in the local blockchain\n", hash)).unwrap();
            ::std::process::exit(1);
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

    term.success(&format!("forward local tip to: {}\n", hash)).unwrap();

    blockchain.save_tip(&hash)
}

pub fn pull( mut term: Term
           , root_dir: PathBuf
           , name: BlockchainName
           )
{
    let blockchain = Blockchain::load(root_dir.clone(), name.clone());

    for np in blockchain.peers() {
        if ! np.is_native() { continue; }
        term.info(&format!("fetching blocks from peer: {}\n", np.name())).unwrap();

        let peer = peer::Peer::prepare(&blockchain, np.name().to_owned());

        peer.connect(&mut term).unwrap().sync(&mut term);
    }

    forward(term, root_dir, name, None)
}

fn get_block(term: &mut Term, blockchain: &Blockchain, hash: &HeaderHash) -> RawBlock
{
    let block_location = match storage::block_location(&blockchain.storage, &hash) {
        None => {
            term.error(&format!("block hash `{}' is not present in the local blockchain\n", hash)).unwrap();
            ::std::process::exit(1);
        },
        Some(loc) => loc
    };

    debug!("blk location: {:?}", block_location);

    match storage::block_read_location(&blockchain.storage, &block_location, &hash) {
        None        => {
            // this is a bug, we have a block location available for this hash
            // but we were not able to read the block.
            panic!("the impossible happened, we have a block location of this given block `{}'", hash)
        },
        Some(rblk) => rblk
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
    let blockchain = Blockchain::load(root_dir.clone(), name.clone());
    let rblk = get_block(term, &blockchain, &hash);

    if no_parse {
        ::std::io::stdout().write(rblk.as_ref())?;
        ::std::io::stdout().flush()?;
    } else {
        use utils::pretty::Pretty;

        let blk = rblk.decode().unwrap();
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
    let blockchain = Blockchain::load(root_dir, name);

    writeln!(term, "Blockchain")?;
    term.warn("Blockchain:\n").unwrap();
    {
        let (tip, _is_genesis) = blockchain.load_tip();
        let tag_path = blockchain.dir.join("tag").join(super::LOCAL_BLOCKCHAIN_TIP_TAG);
        let metadata = ::std::fs::metadata(tag_path).unwrap();
        let fetched_date = metadata.modified().unwrap().into();
        // get the difference between now and the last fetch, only keep up to the seconds
        let fetched_since = time::Duration::since(fetched_date);

        writeln!(term, " * last forward:    {} ({} ago)", style!(fetched_date), style!(fetched_since))?;
        writeln!(term, " * local tip hash:  {}", style!(tip.hash))?;
        writeln!(term, " * local tip date:  {}", style!(tip.date))?;
    }

    writeln!(term, "Peers:")?;
    for (idx, np) in blockchain.peers().enumerate() {
        let peer = peer::Peer::prepare(&blockchain, np.name().to_owned());
        let (tip, _is_genesis) = peer.load_local_tip();

        writeln!(term, "{}. {} ({})", style!(idx+1), style!(peer.name), style!(peer.config))?;

        let tag_path = blockchain.dir.join("tag").join(&peer.tag);
        let metadata = ::std::fs::metadata(tag_path).unwrap();
        let fetched_date = metadata.modified().unwrap().into();
        // get the difference between now and the last fetch, only keep up to the seconds
        let fetched_since = time::Duration::since(fetched_date);

        writeln!(term, " * last fetch:      {} ({} ago)", style!(fetched_date), style!(fetched_since))?;
        writeln!(term, " * local tip hash:  {}", style!(tip.hash))?;
        writeln!(term, " * local tip date:  {}", style!(tip.date))?;
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
    let blockchain = Blockchain::load(root_dir, name);
    let rblk = get_block(term, &blockchain, &hash);
    match rblk.decode() {
        Ok(blk) => {
            match cardano::block::verify_block(blockchain.config.protocol_magic, &hash, &blk) {
                Ok(()) => {
                    writeln!(term, "{}", style!("Block is valid").green())?;
                }
                Err(err) => {
                    writeln!(term, "{}", style!("Invalid block").red())?;
                    term.simply(&format!("{:?}", err)).unwrap();
                    term.simply("\n").unwrap();
                    ::std::process::exit(1);
                }
            };
        },
        Err(err) => {
            term.error("Error: ").unwrap();
            term.simply(&format!("{:?}", err)).unwrap();
            term.simply("\n").unwrap();
            ::std::process::exit(1);
        }
    }
}

pub fn verify_chain( mut term: Term
                   , root_dir: PathBuf
                   , name: BlockchainName
                   )
{
    let blockchain = Blockchain::load(root_dir, name);

    let tip = blockchain.load_tip().0;
    let num_blocks = tip.date.slot_number();

    let progress = term.progress_bar(num_blocks as u64);
    progress.set_message("verifying blocks... ");

    let genesis_data = genesis_data::get_genesis_data(&blockchain.config.genesis_prev)
        .expect("Could not find genesis data.");

    let genesis_data = parse_genesis_data::parse_genesis_data(genesis_data);

    assert_eq!(genesis_data.genesis_prev, blockchain.config.genesis_prev,
            "Genesis data hash mismatch.");

    let mut bad_blocks = 0;
    let mut nr_blocks = 0;
    let mut chain_state = cardano::block::ChainState::new(&genesis_data);

    for res in blockchain.iter_to_tip(blockchain.config.genesis.clone()).unwrap() {
        let (_raw_blk, blk) = res.unwrap();
        nr_blocks += 1;
        let hash = blk.get_header().compute_hash();
        match chain_state.verify_block(&hash, &blk) {
            Ok(()) => {},
            Err(err) => {
                bad_blocks += 1;
                term.error(&format!("Block {} ({}) is invalid: {:?}", hash, blk.get_header().get_blockdate(), err)).unwrap();
                term.simply("\n\n").unwrap();
            }
        }
        progress.inc(1);
    }

    progress.finish();

    term.simply(&format!("{} transactions, {} spent outputs, {} unspent outputs\n",
                         chain_state.nr_transactions,
                         chain_state.spend_txos,
                         chain_state.utxos.len())).unwrap();

    if bad_blocks > 0 {
        term.error(&format!("{} out of {} blocks are invalid", bad_blocks, nr_blocks)).unwrap();
        term.simply("\n").unwrap();
        ::std::process::exit(1);
    }

    term.success(&format!("All {} blocks are valid", nr_blocks)).unwrap();
    term.simply("\n").unwrap();
}
