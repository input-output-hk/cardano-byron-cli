extern crate dirs;
extern crate cardano;
extern crate log;
extern crate env_logger;
extern crate storage_units;
extern crate cardano_storage;

//use self::cardano_cli::utils::term;
//use self::cardano_cli::{blockchain, wallet, debug};
use std::path::PathBuf;

use std::env;
use std::convert::From;
use cardano_storage::config::StorageConfig;
use cardano_storage::epoch;
use cardano_storage::pack::{packreader_init, packreader_block_next};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::cmp::Ordering;

use cardano::{address::Addr, util::base58};
use cardano::util::try_from_slice::TryFromSlice;
use cardano::tx;
use cardano::block::BlockDate;
use cardano::coin::Coin;
use cardano::coin;
use std::ops::Add;

/*
#[derive(Clone)]
pub struct Fund {
    coin: Coin,
    other_coin: Coin,
}

impl Fund {
    pub fn zero() -> Self {
        Fund { coin: Coin::zero(), other_coin: Coin::zero() }
    }
}

impl Add for Fund {
    type Output = Fund;
    fn add(self, fund: Fund) -> Self::Output {
        Fund { coin: (self.coin + fund.coin).unwrap(), other_coin: (self.other_coin + fund.other_coin).unwrap() }
    }
}

pub fn remove_fund(fund: &mut Fund, coin: Coin) -> Fund {
    if fund.other_coin >= coin {
        fund.other_coin = (fund.other_coin - coin).unwrap();
        Fund { coin : Coin::zero(), other_coin : coin }
    } else {
        let total = (fund.coin + fund.other_coin).unwrap();
        let r = Fund { coin : (coin - fund.other_coin).unwrap(), other_coin : fund.other_coin };
        fund.coin = (total - coin).unwrap();
        fund.other_coin = Coin::zero();
        r
    }
}

pub fn add_tracked_fund(fund: &mut Fund, coin: Coin) {
    fund.coin = (fund.coin + coin).unwrap();
}

pub fn add_other_fund(fund: &mut Fund, coin: Coin) {
    fund.other_coin = (fund.other_coin + coin).unwrap();
}

/*
pub enum LogEvent {
    Received(),
}

pub struct Log {
    //hash: BlockHash,
    date: BlockDate,
    event: LogEvent,
}
pub struct Wallet {
    sum: Coin,
    tracked_utxo: BTreeMap<tx::TxIn, Coin>,
    other_utxo: BTreeMap<tx::TxIn, Coin>,
    logs: Vec<Log>,
}

#[derive(Debug)]
pub enum Spent { Tracked, Untracked, }

impl fmt::Display for Wallet {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.sum)
    }
}

impl Wallet {
    pub fn new() -> Self {
        Wallet { sum : Coin::zero(),
                 tracked_utxo : BTreeMap::new(),
                 other_utxo : BTreeMap::new(),
                 logs : Vec::new(),
               }
    }

    pub fn receive_utxo(&mut self, txin: tx::TxIn, value: Coin, to_track: bool) {
        if to_track {
            self.tracked_utxo.insert(txin, value);
        } else {
            self.other_utxo.insert(txin, value);
        }
        self.sum = (self.sum + value).unwrap()
    }

    pub fn spend_utxo(&mut self, txin: &tx::TxIn) -> Option<(Spent, Coin)> {
        match self.tracked_utxo.remove(txin) {
            None       => {
                match self.other_utxo.remove(txin) {
                    None => None,
                    Some(coin) => {
                        self.sum = (self.sum - coin).unwrap();
                        Some((Spent::Untracked, coin))
                    }
                }
            },
            Some(coin) => {
                self.sum = (self.sum - coin).unwrap();
                Some((Spent::Tracked, coin))
            },
        }
    }
}
*/

#[derive(Debug, Clone)]
pub struct TxRevPtr {
    tx_value: Coin,
    tx_tracked_ratio: f64,
    tx_parent: Vec<tx::TxIn>,
}

pub struct CoinTracker {
    total_input: Coin,
    //tracked_input: Coin,
    total_output: Coin,
}

pub struct World {
    utxo: BTreeMap<tx::TxIn, tx::TxOut>,
    all_utxos: BTreeMap<tx::TxIn, TxRevPtr>,
    interesting_utxos: BTreeSet<tx::TxIn>,
    initial_address: Addr,
    pub total: Coin,
    pub redeems: usize,
}

fn coin_ratio(num: Coin, denum: Coin) -> f64 {
    let n = u64::from(num);
    let d = u64::from(denum);
    (n as f64) / (d as f64)
}

impl World {
    pub fn new(addr: Addr) -> Self {
        World {
            utxo: BTreeMap::new(),
            redeems: 0,
            all_utxos: BTreeMap::new(),
            interesting_utxos: BTreeSet::new(),
            total: Coin::zero(),
            initial_address: addr,
        }
    }

    pub fn get_number_utxo(&self) -> usize { self.utxo.len() }
    pub fn get_number_tracked(&self) -> usize { self.all_utxos.len() }
    pub fn get_number_interesting(&self) -> usize { self.interesting_utxos.len() }

    // add a tracked asset
    pub fn add_interesting(&mut self, utxo: tx::TxIn, out: tx::TxOut, spent: Vec<tx::TxIn>, tracker: &CoinTracker) {
        //let input_ratio = coin_ratio(tracker.tracked_input, tracker.total_input);
        let output_ratio = coin_ratio(out.value, tracker.total_output);

        // spent
        let mut total_tracked_spent = Coin::zero();
        if spent.len() == 0 {
            total_tracked_spent = out.value;
        }
        // calculate how much tracked / untracked asset there is in input
        for i in spent.iter() {
            match self.all_utxos.get(i) {
                None => {
                    // if we don't have the utxo, assumed, 100% untracked asset
                },
                Some(x) => {
                    let z = coin::scale(x.tx_value, x.tx_tracked_ratio).unwrap();
                    total_tracked_spent = (total_tracked_spent + z).unwrap();
                },
            }
        }

        let total_input_tracked = (tracker.total_input - total_tracked_spent).unwrap();
        let total_input_untracked = (tracker.total_input - total_input_tracked).unwrap();
        //let total_output_tracked = coin::scale(total_input_tracked, output_ratio);
        //let total_output_untracked = coin::scale(total_input_untracked, output_ratio);
        let ratio = coin_ratio(total_tracked_spent, tracker.total_input);

        //println!("total input {} total_spent: {} ratio {} value {}", tracker.total_input, total_tracked_spent, ratio, out.value);

        let txrp = TxRevPtr {
            tx_value: out.value,
            tx_tracked_ratio: ratio,
            tx_parent: spent.clone(),
        };

        self.all_utxos.insert(utxo.clone(), txrp);
        self.interesting_utxos.insert(utxo);
        ()
    }

    pub fn dump_interesting(&self) -> Vec<(tx::TxIn, TxRevPtr)> {
        let mut out = Vec::new();
        for utxo in self.interesting_utxos.iter() {
            match self.all_utxos.get(utxo) {
                None => { println!("XXZZZa") },
                Some(trp) => out.push((utxo.clone(), (*trp).clone())),
            };
        }
        out
    }

    // compare all utxo to the one we're tracking
    pub fn transaction(&mut self, trans: &tx::TxAux) {
        let ref tx = trans.tx;
        let txid = trans.tx.id();

        let mut spent = Vec::new();
        let mut interesting = false;

        let mut input_coins = Coin::zero();
        //let mut tracked_coins = Coin::zero();

        let nb_inputs = tx.inputs.len();
        let nb_outputs = tx.outputs.len();

        for (iidx, ic) in tx.inputs.iter().enumerate() {
            match self.utxo.remove(ic) {
                None => {
                    if let tx::TxInWitness::RedeemWitness(_, _) = (*trans.witness)[iidx] {
                        self.redeems += 1
                    } else {
                        println!("oh noes. something wrong !?: {:?} {:?}", ic, (*trans.witness)[iidx])
                    }
                },
                Some(utxo_txout) => {
                    input_coins = (input_coins + utxo_txout.value).unwrap();
                    if self.interesting_utxos.remove(ic) {
                        //tracked_coins = (tracked_coins + utxo_txout.value).unwrap();
                        interesting = true;
                        //spent.push((ic.clone(), utxo_txout));
                        spent.push(ic.clone());
                    };
                },
            }
        }

        let output_total = {
            let mut total = Coin::zero();
            for o in tx.outputs.iter() {
                total = (total + o.value).unwrap()
            };
            total
        };

        let ratio = CoinTracker { total_input: input_coins, total_output: output_total };
        for (oidx, o) in tx.outputs.iter().enumerate() {
            let utxo = tx::TxIn::new(txid.clone(), oidx as u32);
            if self.initial_address == o.address.clone().into() {
                if nb_inputs == 1 && nb_outputs == 1 {
                    let sratio = CoinTracker { total_input: o.value, total_output: o.value, };
                    self.total = (self.total + o.value).unwrap();
                    self.add_interesting(utxo.clone(), o.clone(), Vec::new(), &sratio);
                } else {
                    println!("AAAAAAAAAAAAAAARG")
                }
            } else {
                if interesting {
                    self.add_interesting(utxo.clone(), o.clone(), spent.clone(), &ratio);
                }
            }
            self.utxo.insert(utxo, o.clone());
        }
    }
}

pub fn main() {
    let args: Vec<String> = env::args().collect();
    let storage_root = PathBuf::from(&args[1]);
    let addr = args[2].clone();
    let cfg = StorageConfig::new(&storage_root);

    let address_raw = base58::decode(&addr).unwrap();
    let addr = Addr::try_from_slice(&address_raw[..]).unwrap();

    println!("storage at : {:?}", storage_root);
    println!("address    : {:?}", addr);

    let known_epochs = {
        let mut epochs = Vec::new();
        let mut i = 0;
        loop {
            match epoch::epoch_read(&cfg, i) {
                Ok(r) => epochs.push(r),
                Err(e) => break,
            }
            i += 1
        }
        epochs
    };

    let initial_address = addr;
    let mut world = World::new(initial_address);

    println!("pack epoch in storage: {}", known_epochs.len());
    let mut epoch_nb = 0;

    for (packref, refpack) in known_epochs.iter() {
        let mut pr = packreader_init(&cfg, packref);
        //let mut block_nb = 0;
        println!("epoch {} tracking .. {} redeems, and {} utxos {} interesting {} tracked", epoch_nb, world.redeems, world.get_number_utxo(), world.get_number_interesting(), world.get_number_tracked());
        while let Ok(rblk) = packreader_block_next(&mut pr) {
            let blk = rblk.decode().unwrap();
            let hdr = blk.get_header();
            match blk.get_transactions() {
                None => {},
                Some(txpayload) => {
                    let mut interesting = false;
                    for trans in txpayload.iter() {
                        world.transaction(&trans);
                    }
                },
            }
            //block_nb += 1
        }

        //println!("packref: {:?}", packref);
        epoch_nb += 1;
        /*
        if epoch_nb > 8 {
            break;
        }
        */
    }

    let mut tracked = Coin::zero();
    let mut result = Vec::new();
    for (utxo, txp) in world.dump_interesting() {
        let adjusted = coin::scale(txp.tx_value, txp.tx_tracked_ratio).unwrap();
        tracked = (tracked + adjusted).unwrap();
        let total_ratio = coin_ratio(adjusted, world.total);
        result.push((utxo, txp.tx_value, adjusted, total_ratio));
        /*
        if total_ratio > 0.05 {
            println!("utxo {}@{} {}/{} coins {} ratio", utxo.id, utxo.index, adjusted, txp.tx_value, total_ratio);
        }
        */
    }
    println!("tracked: {} expecting: {}", tracked, world.total);

    result.sort_unstable_by(|a, b| a.3.partial_cmp(&b.3).unwrap_or(Ordering::Greater));

    for (utxo, value, adjusted, total_ratio) in result.iter() {
        println!("utxo {}@{} {}/{} coins {} ratio", utxo.id, utxo.index, adjusted, value, total_ratio);
    }

    //for (k,v) in world.wallets.iter() {
    //    println!("address {} = {}", k, v.sum)
    //}
}
*/

pub struct World {
    pub nb_address: usize,
    pub address_total_size: usize,
    pub nb_witnesses: usize,
}

impl World {
    pub fn new() -> Self {
        World {
            nb_address: 0,
            address_total_size: 0,
            nb_witnesses: 0,
        }
    }

    pub fn transaction(&mut self, trans: &tx::TxAux) {
        let tx = &trans.tx;
        for o in tx.outputs.iter() {
            let a : Addr = o.address.clone().into();
            let x = a.as_ref().len();
            self.nb_address += 1;
            self.address_total_size += x;
        }
        self.nb_witnesses += trans.witness.len();
    }
}

pub fn main() {
    let args: Vec<String> = env::args().collect();
    let storage_root = PathBuf::from(&args[1]);
    let cfg = StorageConfig::new(&storage_root);

    println!("storage at : {:?}", storage_root);

    let known_epochs = {
        let mut epochs = Vec::new();
        let mut i = 0;
        loop {
            match epoch::epoch_read(&cfg, i) {
                Ok(r) => epochs.push(r),
                Err(e) => break,
            }
            i += 1
        }
        epochs
    };

    let mut world = World::new();

    println!("pack epoch in storage: {}", known_epochs.len());
    let mut epoch_nb = 0;

    for (packref, refpack) in known_epochs.iter() {
        let mut pr = packreader_init(&cfg, packref);
        //let mut block_nb = 0;
        println!("epoch {}", epoch_nb);
        while let Ok(Some(rblk)) = packreader_block_next(&mut pr) {
            let blk = rblk.decode().unwrap();
            //let hdr = blk.get_header();
            match blk.get_transactions() {
                None => {},
                Some(txpayload) => {
                    for trans in txpayload.iter() {
                        world.transaction(&trans)
                    }
                }
            }
            //block_nb += 1
        }

        epoch_nb += 1;
        //if epoch_nb > 8 {
        //    break;
        //}
        println!("   addresses: {}", world.nb_address);
        println!("   size_addr: {}", world.address_total_size);
        println!("   witnesses: {}", world.nb_witnesses);
    }

    println!("addresses: {}", world.nb_address);
    println!("size_addr: {}", world.address_total_size);
    println!("witnesses: {}", world.nb_witnesses);

    /*
    let mut tracked = Coin::zero();
    let mut result = Vec::new();
    for (utxo, txp) in world.dump_interesting() {
        let adjusted = coin::scale(txp.tx_value, txp.tx_tracked_ratio).unwrap();
        tracked = (tracked + adjusted).unwrap();
        let total_ratio = coin_ratio(adjusted, world.total);
        result.push((utxo, txp.tx_value, adjusted, total_ratio));
        /*
        if total_ratio > 0.05 {
            println!("utxo {}@{} {}/{} coins {} ratio", utxo.id, utxo.index, adjusted, txp.tx_value, total_ratio);
        }
        */
    }
    println!("tracked: {} expecting: {}", tracked, world.total);

    result.sort_unstable_by(|a, b| a.3.partial_cmp(&b.3).unwrap_or(Ordering::Greater));

    for (utxo, value, adjusted, total_ratio) in result.iter() {
        println!("utxo {}@{} {}/{} coins {} ratio", utxo.id, utxo.index, adjusted, value, total_ratio);
    }

    //for (k,v) in world.wallets.iter() {
    //    println!("address {} = {}", k, v.sum)
    //}
*/
}
