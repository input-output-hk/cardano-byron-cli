pub mod commands;
pub mod config;
pub mod error;
pub mod peer;

pub use self::error::{Error, Result};

use std::{
    ffi::OsString,
    fmt,
    ops::Deref,
    path::{Path, PathBuf},
    str::FromStr,
};

use cardano::block;
use cardano_storage::{self as storage, config::StorageConfig, tag, Storage};
use exe_common::network::api::BlockRef;
pub use exe_common::{
    config::net::{self, Config, Peer, Peers},
    genesisdata, network,
};
use storage_units::utils::directory_name::{DirectoryName, DirectoryNameError};

pub const LOCAL_BLOCKCHAIN_TIP_TAG: &'static str = "tip";

pub type BlockchainNameError = DirectoryNameError;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BlockchainName(DirectoryName);
impl BlockchainName {
    pub fn from_os_str(os: OsString) -> ::std::result::Result<Self, DirectoryNameError> {
        DirectoryName::new(os).map(BlockchainName)
    }
}
impl FromStr for BlockchainName {
    type Err = <DirectoryName as FromStr>::Err;
    fn from_str(s: &str) -> ::std::result::Result<Self, Self::Err> {
        DirectoryName::from_str(s).map(BlockchainName)
    }
}
impl fmt::Display for BlockchainName {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}
impl Deref for BlockchainName {
    type Target = <DirectoryName as Deref>::Target;
    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}
impl AsRef<str> for BlockchainName {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

/// handy structure to use to manage and orginise a blockchain
///
pub struct Blockchain {
    pub name: BlockchainName,
    pub dir: PathBuf,
    pub storage_config: StorageConfig,
    pub storage: Storage,
    pub config: Config,
}
impl Blockchain {
    /// create the new blockhain with the given setting
    pub fn new(root_dir: PathBuf, name: BlockchainName, config: Config) -> Result<Self> {
        let dir = config::directory(root_dir, &name);
        let storage_config = StorageConfig::new(&dir);

        debug!("init storage");
        let storage = Storage::init(&storage_config)
            .map_err(Error::NewCannotInitializeBlockchainDirectory)?;
        let file = storage_config.get_config_file();
        debug!("creating config file");
        config.to_file(file);

        // by default, the config file comes with pre-set remote peers,
        // check that, for every peer, we add them to the fold
        for peer in config.peers.iter() {
            let tag = format!("remote/{}", peer.name());
            tag::write_hash(&storage, &tag, &config.genesis)
        }
        debug!("wrote initial hashes");

        let blockchain = Blockchain {
            name,
            dir,
            storage_config,
            storage,
            config,
        };

        debug!("saving initial Tip");
        blockchain.save_tip(&blockchain.config.genesis);
        debug!("initializing genesis data");
        blockchain.init_genesis_data()?;
        debug!("done...");

        Ok(blockchain)
    }

    fn init_genesis_data(&self) -> Result<()> {
        use std::{fs::OpenOptions, io::Write};
        let genesis_data = genesisdata::data::get_genesis_data(&self.config.genesis_prev)
            .map_err(Error::VerifyChainGenesisHashNotFound)?;

        let path = self.dir.join("genesis.json");

        debug!("writing genesis file: {:?}", path);
        let mut fs = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)?;

        fs.write_all(genesis_data.as_bytes())?;
        Ok(())
    }

    pub fn load_genesis_data(&self) -> Result<cardano::config::GenesisData> {
        use std::fs::OpenOptions;
        let path = self.dir.join("genesis.json");
        let fs = OpenOptions::new().read(true).open(path)?;

        let genesis_data = genesisdata::parse::parse(fs);

        Ok(genesis_data)
    }

    pub unsafe fn destroy(self) -> ::std::io::Result<()> {
        ::std::fs::remove_dir_all(self.dir)
    }

    /// load the blockchain
    pub fn load<P>(root_dir: P, name: BlockchainName) -> Result<Self>
    where
        P: AsRef<Path>,
    {
        Self::load_internal(root_dir.as_ref(), name)
    }

    fn load_internal(root_dir: &Path, name: BlockchainName) -> Result<Self> {
        let dir = config::directory(root_dir, &name);
        let storage_config = StorageConfig::new(&dir);
        let storage = Storage::init(&storage_config)?;

        let file = storage_config.get_config_file();
        let config = match Config::from_file(&file) {
            Some(config) => config,
            None => {
                return Err(Error::LoadConfigFileNotFound(file));
            }
        };

        let blockchain = Blockchain {
            name,
            dir,
            storage_config,
            storage,
            config,
        };

        // compatibility with previously generated blockchain
        let genesis_file = blockchain.dir.join("genesis.json");
        if !genesis_file.exists() {
            blockchain.init_genesis_data()?;
        }

        Ok(blockchain)
    }

    /// save the blockchain settings
    pub fn save(&self) {
        self.config.to_file(self.storage_config.get_config_file());
    }

    /// add a peer to the blockchain
    pub fn add_peer(&mut self, remote_alias: String, remote_endpoint: String) {
        let tag = self.mk_remote_tag(&remote_alias);

        let peer = Peer::new(remote_endpoint);
        self.config.peers.push(remote_alias, peer);

        tag::write_hash(&self.storage, &tag, &self.config.genesis)
    }

    pub fn mk_remote_tag(&self, remote: &str) -> String {
        format!("remote/{}", remote)
    }

    pub fn load_remote_tips(&self) -> Vec<(BlockRef, bool)> {
        self.peers()
            .map(|np| {
                let peer = peer::Peer::prepare(self, np.name().to_owned());
                peer.load_local_tip()
            })
            .collect()
    }

    /// remove a peer from the blockchain
    pub fn remove_peer(&mut self, remote_alias: String) {
        self.config.peers = self
            .config
            .peers
            .iter()
            .filter(|np| np.name() != remote_alias)
            .cloned()
            .collect();
        let tag = self.mk_remote_tag(&remote_alias);
        tag::remove_tag(&self.storage, &tag);
    }

    pub fn peers<'a>(&'a self) -> impl Iterator<Item = &'a net::NamedPeer> {
        self.config.peers.iter()
    }

    pub fn load_tip(&self) -> (BlockRef, bool) {
        let genesis_ref = (
            BlockRef {
                hash: self.config.genesis.clone(),
                parent: self.config.genesis_prev.clone(),
                date: block::BlockDate::Boundary(self.config.epoch_start),
            },
            true,
        );
        match self.storage.get_block_from_tag(LOCAL_BLOCKCHAIN_TIP_TAG) {
            Err(storage::Error::NoSuchTag) => genesis_ref,
            Err(err) => panic!(err),
            Ok(block) => {
                let header = block.get_header();
                let hash = header.compute_hash();
                let is_genesis = hash == genesis_ref.0.hash;
                (
                    BlockRef {
                        hash: hash,
                        parent: header.get_previous_header(),
                        date: header.get_blockdate(),
                    },
                    is_genesis,
                )
            }
        }
    }
    pub fn save_tip(&self, hh: &block::HeaderHash) {
        tag::write_hash(&self.storage, &LOCAL_BLOCKCHAIN_TIP_TAG, hh);
    }

    pub fn iter<'a>(
        &'a self,
        from: block::HeaderHash,
        to: block::HeaderHash,
    ) -> storage::Result<storage::iter::Iter<'a>> {
        storage::iter::Iter::new(&self.storage, from, to)
    }

    pub fn iter_to_tip<'a>(
        &'a self,
        from: block::HeaderHash,
    ) -> storage::Result<storage::iter::Iter<'a>> {
        let to = self.load_tip().0.hash;

        self.iter(from, to)
    }
}
