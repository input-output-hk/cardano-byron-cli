use std::path::PathBuf;

use exe_common::{config::net::{Config, Peer, Peers}, sync, network};
use exe_common::network::api::BlockRef;


use utils::term::Term;

use super::peer;
use super::Blockchain;

/// function to create and initialise a given new blockchain
///
/// It will mainly create the subdirectories needed for the storage
/// of blocks, epochs and tags.
///
/// If the given blockchain configuration provides some preset peers
/// each peer will be initialised with an associated tag pointing to
/// the genesis hash of the blockchain (given in the same configuration
/// structure `Config`).
///
pub fn new( mut term: Term
          , root_dir: PathBuf
          , name: String
          , config: Config
          )
{
    let blockchain = Blockchain::new(root_dir, name.clone(), config);
    blockchain.save();

    term.success(&format!("local blockchain `{}' created.\n", &name)).unwrap();
}

/// function to add a remote to the given blockchain
///
/// It will create the appropriate tag refering to the blockchain
/// genesis hash. This is because when add a new peer we don't assume
/// anything more than the genesis block.
///
pub fn remote_add( mut term: Term
                 , root_dir: PathBuf
                 , name: String
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
                , name: String
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
                   , name: String
                   , peers: Vec<String>
                   )
{
    let mut blockchain = Blockchain::load(root_dir, name);

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
                , name: String
                , detailed: RemoteDetail
                )
{
    let blockchain = Blockchain::load(root_dir, name);

    for np in blockchain.peers() {
        let peer = peer::Peer::prepare(&blockchain, np.name().to_owned());
        let (tip, _is_genesis) = peer.load_local_tip();

        term.info(&format!("{}", peer.name)).unwrap();
        term.simply(" (").unwrap();
        term.success(&format!("{}", peer.config)).unwrap();
        term.simply(")\n").unwrap();

        if detailed >= RemoteDetail::Local {
            let tag_path = blockchain.dir.join("tag").join(&peer.tag);
            let metadata = ::std::fs::metadata(tag_path).unwrap();
            let now = ::std::time::SystemTime::now();
            let fetched_date = metadata.modified().unwrap();
            // get the difference between now and the last fetch, only keep up to the seconds
            let fetched_since = ::std::time::Duration::new(now.duration_since(fetched_date).unwrap().as_secs(), 0);

            term.simply(" * last fetch:      ").unwrap();
            term.info(&format!("{} ({} ago)", format_systemtime(fetched_date), format_duration(fetched_since))).unwrap();
            term.simply("\n").unwrap();
            term.simply(" * local tip hash:  ").unwrap();
            term.success(&format!("{}", tip.hash)).unwrap();
            term.simply("\n").unwrap();
            term.simply(" * local tip date:  ").unwrap();
            term.success(&format!("{}", tip.date)).unwrap();
            term.simply("\n").unwrap();

            if detailed >= RemoteDetail::Remote {
                let mut connected_peer = peer.connect(&mut term).unwrap();
                let remote_tip = connected_peer.query_tip();
                let block_diff = remote_tip.date - tip.date;

                term.simply(" * remote tip hash: ").unwrap();
                term.warn(&format!("{}", remote_tip.hash)).unwrap();
                term.simply("\n").unwrap();
                term.simply(" * remote tip date: ").unwrap();
                term.warn(&format!("{}", remote_tip.date)).unwrap();
                term.simply("\n").unwrap();
                term.simply(" * local is ").unwrap();
                term.warn(&format!("{}", block_diff)).unwrap();
                term.simply(" blocks behind remote\n").unwrap();
            }
        }
    }
}

fn format_systemtime(time: ::std::time::SystemTime) -> String {
    format!("{}", ::humantime::format_rfc3339(time)).chars().take(10).collect()
}
fn format_duration(duration: ::std::time::Duration) -> String {
    format!("{}", ::humantime::format_duration(duration))
}

pub fn forward( mut term: Term
              , root_dir: PathBuf
              , name: String
              , to: Option<String>
              )
{
    let mut blockchain = Blockchain::load(root_dir, name);

    let hash = if let Some(hash_hex) = to {
        let hash = match ::cardano::util::hex::decode(&hash_hex) {
            Ok(hash) => match ::cardano::block::HeaderHash::from_slice(hash.as_ref()) {
                Err(err) => {
                    debug!("invalid block hash: {}", err);
                    term.error(&format!("invalid hash `{}': this is not a valid block hash\n", hash_hex)).unwrap();
                    ::std::process::exit(1);
                },
                Ok(hash) => hash
            },
            Err(err) => {
                debug!("invalid block hash: {:?}", err);
                term.error(&format!("invalid hash `{}': invalid hexadecimal\n", hash_hex)).unwrap();
                ::std::process::exit(1);
            }
        };

        if ::storage::block_location(&blockchain.storage, hash.bytes()).is_none() {
            term.error(&format!("block hash `{}' is not present in the local blockchain\n", hash_hex)).unwrap();
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
