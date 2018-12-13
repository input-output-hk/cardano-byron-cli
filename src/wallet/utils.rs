//! Wallet utilities
//!
//! provides function for the wallet commands (and other command groups) to
//! manipulate wallets, load filter, or even create.
//!

use super::{Wallet};
use super::config::{Config, HDWalletModel};
use super::error::{Error, Result};
use super::state::{log, ptr, state, lookup, iter::TransactionIterator, utxo::UTxO, ptr::{StatePtr}};

use cardano::{address::ExtendedAddr, block::{BlockDate}, config::{ProtocolMagic, NetworkMagic}, tx::{TxInWitness, TxId}};

use utils::{term::{Term, style::{Style}}};

use blockchain::Blockchain;

use std::{
    io::Write,
    path::Path,
};

pub fn update_wallet_state_with_utxos<LS>( term: &mut Term
                                         , wallet: &Wallet
                                         , blockchain: &Blockchain
                                         , state: &mut state::State<LS>
                                         )
    where LS: lookup::AddressLookup
{
    let blockchain_tip = blockchain.load_tip().0;

    let from_ptr = state.ptr().clone();
    let from = from_ptr.latest_known_hash;
    let from_date = from_ptr.latest_addr.unwrap_or(BlockDate::Boundary(0));
    let num_blocks = blockchain_tip.date - from_date;

    term.info(&format!("syncing wallet from {} to {}\n", from_date, blockchain_tip.date)).unwrap();

    let progress = term.progress_bar(num_blocks as u64);
    progress.set_message("loading transactions... ");

    let mut last_block_date = from_date;
    for res in TransactionIterator::new(progress, blockchain.iter_to_tip(from).unwrap_or_else(|e| term.fail_with(e)) ) {
        let (ptr, txaux) = res.unwrap_or_else(|e| term.fail_with(e));
        debug!("transactions in: {}", ptr);

        if let Some(addr) = ptr.latest_addr {
            // Ignore the latest known block hash so next call to sync won't fail.
            if addr == from_date { continue; }
            if last_block_date.get_epochid() != addr.get_epochid() {

                let log_lock = lock_wallet_log(&wallet);
                let mut writer = log::LogWriter::open(log_lock).unwrap_or_else(|e| term.fail_with(e));
                let log : log::Log<ExtendedAddr> = log::Log::Checkpoint(ptr.clone());
                writer.append(&log).unwrap_or_else(|e| term.fail_with(e));
            }

            last_block_date = addr.clone();
        }

        {
            let logs = state.forward_with_txins(
                txaux.tx.inputs.iter().map(|txin| (ptr.clone(), txin))
            ).unwrap_or_else(|e| term.fail_with(e));
            let log_lock = lock_wallet_log(&wallet);
            let mut writer = log::LogWriter::open(log_lock).unwrap_or_else(|e| term.fail_with(e));
            for log in logs { writer.append(&log).unwrap_or_else(|e| term.fail_with(e)); }
        }

        {
            let txid = txaux.tx.id();
            let logs = state.forward_with_utxos(
                txaux.tx.outputs.into_iter().enumerate().map(|(idx, txout)| {
                    ( ptr.clone()
                    , UTxO {
                        transaction_id: txid.clone(),
                        index_in_transaction: idx as u32,
                        credited_address: txout.address.clone(),
                        credited_addressing: txout.address,
                        credited_value: txout.value
                      }
                    )
                })
            ).unwrap_or_else(|e| term.fail_with(e));

            let log_lock = lock_wallet_log(&wallet);
            let mut writer = log::LogWriter::open(log_lock).unwrap_or_else(|e| term.fail_with(e));
            for log in logs { writer.append(&log).unwrap_or_else(|e| term.fail_with(e)); }
        }
    }
}

pub fn display_wallet_state_utxos<LS>( term: &mut Term
                                     , state: state::State<LS>
                                     )
    where LS: lookup::AddressLookup
{
    for (_, utxo) in state.utxos {
        writeln!(term, "{}.{} {}",
            style!(utxo.transaction_id),
            style!(utxo.index_in_transaction).yellow(),
            style!(utxo.credited_value).green()
        ).unwrap()
    }
}

pub fn display_wallet_state_logs<LS>( term: &mut Term
                                    , wallet: &Wallet
                                    , _state: &mut state::State<LS>
                                    , pretty: bool
                                    )
    where LS: lookup::AddressLookup
{
    let log_lock = lock_wallet_log(&wallet);
    let reader = log::LogReader::open(log_lock).unwrap();
    let reader : log::LogIterator<lookup::Address> = reader.into_iter();
    let reader = reader.filter_map(|r| {
        match r {
            Err(err) => {
                panic!("{:?}", err)
            },
            Ok(v) => Some(v)
        }
    });

    for log in reader {
        match log {
            log::Log::Checkpoint(ptr) => {
                if ! pretty {
                    writeln!(term, "{} {} ({})",
                        style!("checkpoint").cyan(),
                        style!(ptr.latest_block_date()),
                        style!(ptr.latest_known_hash)
                    ).unwrap();
                    writeln!(term, "").unwrap();
                }
            },
            log::Log::ReceivedFund(ptr, utxo) => {
                if pretty {
                    display_utxo(term, ptr, utxo, false);
                } else {
                    dump_utxo(term, ptr, utxo, false);
                }
            },
            log::Log::SpentFund(ptr, utxo) => {
                if pretty {
                    display_utxo(term, ptr, utxo, true);
                } else {
                    dump_utxo(term, ptr, utxo, true);
                }
            }
        }
    }
}

pub fn display_utxo<L>(term: &mut Term, ptr: StatePtr, utxo: UTxO<L>, debit: bool) {
    let ptr = format!("{}", style!(ptr.latest_block_date()));
    let tid = format!("{}", style!(utxo.transaction_id));
    let tii = format!("{:03}", utxo.index_in_transaction);
    const WIDTH : usize = 14;
    let credit = if debit {
        format!("{:>width$}", " ", width = WIDTH)
    } else {
        format!("{:>width$}", format!("{}", utxo.credited_value), width = WIDTH)
    };
    let debit = if debit {
        format!("{:>width$}", format!("{}", utxo.credited_value), width = WIDTH)
    } else {
        format!("{:>width$}", " ", width = WIDTH)
    };

    writeln!(term, "{:9}|{}.{}|{}|{}",
        ::console::pad_str(&ptr, 9, ::console::Alignment::Left, None),
        tid,
        style!(tii).yellow(),
        style!(credit).green(),
        style!(debit).red()
    ).unwrap()
}

pub fn dump_utxo<L>(term: &mut Term, ptr: StatePtr, utxo: UTxO<L>, debit: bool) {
    let title = if debit {
        style!("debit").red()
    } else {
        style!("credit").green()
    };
    let amount = if debit {
        style!(format!("{}", utxo.credited_value)).red()
    } else {
        style!(format!("{}", utxo.credited_value)).green()
    };

    writeln!(term, "{} {}.{}",
        title,
        style!(utxo.transaction_id),
        style!(utxo.index_in_transaction).yellow(),
    ).unwrap();
    writeln!(term, "Date {}", style!(ptr.latest_block_date())).unwrap();
    writeln!(term, "Block {}", style!(ptr.latest_known_hash)).unwrap();
    writeln!(term, "Value {}", amount).unwrap();
    writeln!(term, "").unwrap()
}

pub fn create_wallet_state_from_logs<P, LS>(
    wallet: &Wallet,
    root_dir: P,
    lookup_structure: LS
) -> Result<state::State<LS>>
where
    P: AsRef<Path>,
    LS: lookup::AddressLookup,
{
    pub use super::state::state::FromLogsError::*;

    let log_lock = lock_wallet_log(wallet);
    let log_reader = log::LogReader::open(log_lock)?;
    let log_iter = log_reader.into_iter();
    let state_res = state::State::from_logs(lookup_structure, log_iter);
    match state_res {
        Ok(state) => Ok(state),
        Err(NoEntries(lookup_structure)) => {
            // create empty state
            // 1. get the wallet's blockchain
            let blockchain = load_attached_blockchain(
                root_dir.as_ref(),
                &wallet.config,
            )?;

            // 2. prepare the wallet state
            let initial_ptr = ptr::StatePtr::new_before_genesis(blockchain.config.genesis.clone());
            Ok(state::State::new(initial_ptr, lookup_structure))
        }
        Err(LogReadFailed(e)) => Err(e.into()),
        Err(AddressLookupFailed(e)) => Err(e.into()),
    }
}

pub fn load_bip44_lookup_structure(term: &mut Term, network_magic: NetworkMagic, wallet: &Wallet) -> lookup::sequentialindex::SequentialBip44Lookup {
    // TODO: to prevent from the need of the password, we can ask the user to create accounts ahead.
    //       if we store the wallet's account public keys in the config file we may not need for the
    //       password (and for the private key).
    term.info("Enter the wallet password.\n").unwrap();
    let password = term.password("wallet password: ").unwrap();

    let wallet = match wallet.get_wallet_bip44(password.as_bytes()) {
        Err(Error::CannotRetrievePrivateKeyInvalidPassword) => {
            term.error("Invalid wallet spending password").unwrap();
            ::std::process::exit(1);
        },
        Err(Error::CannotRetrievePrivateKey(err)) => {
            term.error(&format!("Cannot retrieve the private key of the wallet: {}", err)).unwrap();
            term.info("The encrypted wallet password is in an invalid format. You might need to delete this wallet and recover it.").unwrap();
            ::std::process::exit(1);
        },
        Err(err) => {
            term.error(IMPOSSIBLE_HAPPENED).unwrap();
            panic!("failing with an unexpected error {:#?}", err);
        },
        Ok(wallet) => { wallet }
    };
    lookup::sequentialindex::SequentialBip44Lookup::new(wallet, network_magic)
}
pub fn load_randomindex_lookup_structure(term: &mut Term, network_magic: NetworkMagic, wallet: &Wallet) -> lookup::randomindex::RandomIndexLookup {
    // in the case of the random index, we may not need the password if we have the public key
    term.info("Enter the wallet password.\n").unwrap();
    let password = term.password("wallet password: ").unwrap();

    let wallet = match wallet.get_wallet_rindex(password.as_bytes()) {
        Err(Error::CannotRetrievePrivateKeyInvalidPassword) => {
            term.error("Invalid wallet spending password").unwrap();
            ::std::process::exit(1);
        },
        Err(Error::CannotRetrievePrivateKey(err)) => {
            term.error(&format!("Cannot retrieve the private key of the wallet: {}", err)).unwrap();
            term.info("The encrypted wallet password is in an invalid format. You might need to delete this wallet and recover it.").unwrap();
            ::std::process::exit(1);
        },
        Err(err) => {
            term.error(IMPOSSIBLE_HAPPENED).unwrap();
            panic!("failing with an unexpected error {:#?}", err);
        },
        Ok(wallet) => { wallet }
    };
    lookup::randomindex::RandomIndexLookup::from_wallet(wallet, network_magic)
}

pub fn lock_wallet_log(wallet: &Wallet) -> log::LogLock {
    match wallet.log() {
        Err(Error::WalletLogAlreadyLocked(pid)) => {
            error!("Wallet's LOG already locked by another process or thread ({})\n", pid);
            ::std::process::exit(1);
        },
        Err(err) => {
            error!("{}", IMPOSSIBLE_HAPPENED);
            panic!("`lock_wallet_log' has failed with an unexpected error {:#?}", err);
        },
        Ok(lock) => { lock }
    }
}

pub fn load_attached_blockchain<P: AsRef<Path>>(
    root_dir: P,
    wallet_config: &Config,
) -> Result<Blockchain> {
    load_attached_blockchain_internal(root_dir.as_ref(), wallet_config)
}

fn load_attached_blockchain_internal(
    root_dir: &Path,
    wallet_config: &Config,
) -> Result<Blockchain> {
    match wallet_config.attached_blockchain()? {
        None => {
            Err(Error::NotAttachedToBlockchain)
        }
        Some(blockchain) => {
            Ok(Blockchain::load(root_dir, blockchain)?)
        }
    }
}

pub fn wallet_sign_tx(term: &mut Term, wallet: &Wallet, protocol_magic: ProtocolMagic, txid: &TxId, address: &lookup::Address) -> TxInWitness
{
    match wallet.config.hdwallet_model {
        HDWalletModel::BIP44 => {
            let wallet = load_bip44_lookup_structure(term, protocol_magic.into(), wallet);
            if let lookup::Address::Bip44(addressing) = address {
                let xprv = wallet.get_private_key(addressing);
                TxInWitness::new(protocol_magic, &*xprv, txid)
            } else {
                panic!()
            }
        },
        HDWalletModel::RandomIndex2Levels => {
            let wallet = load_randomindex_lookup_structure(term, protocol_magic.into(), wallet);
            if let lookup::Address::RIndex(addressing) = address {
                let xprv = wallet.get_private_key(addressing);
                TxInWitness::new(protocol_magic, &xprv, txid)
            } else {
                panic!()
            }
        },
    }
}

const IMPOSSIBLE_HAPPENED : &'static str = "The impossible happened
The process will panic with an error message, this is because something
unexpected happened. Please report the error message with the panic
error message to: https://github.com/input-output-hk/rust-cardano/issues
";
