use std::{path::PathBuf, io::Write, iter, collections::BTreeMap};
use utils::term::{Term, style::{Style}};
use super::core::{self, transaction, StagingId, StagingTransaction};
use super::super::blockchain::{Blockchain};
use super::super::wallet::{Wallets, Wallet, self, WalletName};
use cardano::{tx::{TxId, TxoPointer, TxInWitness}, coin::{Coin, sum_coins}, address::{ExtendedAddr}, fee::{LinearFee, FeeAlgorithm}};
use cardano::tx;
use storage_units;

pub enum Error {
    IoError(::std::io::Error),
    CannotCreateNewTransaction(storage_units::append::Error),
}
impl From<::std::io::Error> for Error {
    fn from(e: ::std::io::Error) -> Self { Error::IoError(e) }
}

/// function to create a new empty transaction
pub fn new( mut term: Term
          , root_dir: PathBuf
          , blockchain: String
          )
    -> Result<(), Error>
{
    let blockchain = Blockchain::load(root_dir.clone(), blockchain);

    let staging = StagingTransaction::new(root_dir, blockchain.config.protocol_magic)
        .map_err(Error::CannotCreateNewTransaction)?;

    writeln!(term, "Staging file successfully created: {}", style!(staging.id()))?;

    Ok(())
}

pub fn list( mut term: Term
           , root_dir: PathBuf
           )
{
    let transactions_dir = core::config::transaction_directory(root_dir.clone());

    for entry in ::std::fs::read_dir(transactions_dir).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_dir() {
            term.warn(&format!("unexpected directory in transaction directory: {:?}", entry.path())).unwrap();
            continue;
        }
        let name = entry.file_name().into_string().unwrap_or_else(|err| {
            panic!("invalid utf8... {:?}", err)
        });

        let staging = load_staging(&mut term, root_dir.clone(), name.as_str());

        writeln!(term, "{}", style!(staging.id())).unwrap();
    }
}

pub fn destroy( mut term: Term
              , root_dir: PathBuf
              , id_str: &str
              )
{
    let staging = load_staging(&mut term, root_dir, id_str);

    if let Err(err) = staging.destroy() {
        error!("{:?}", err);
        term.error("cannot delete the sta").unwrap();
    } else {
        term.success("transaction deleted\n").unwrap();
    }
}

/// function to create a new empty transaction
pub fn send( mut term: Term
           , root_dir: PathBuf
           , id_str: &str
           , blockchain: String
           )
{
    let staging = load_staging(&mut term, root_dir.clone(), id_str);
    let blockchain = Blockchain::load(root_dir.clone(), blockchain);

    let (finalized, changes) = staging.transaction().mk_finalized().unwrap_or_else(|e| term.fail_with(e));
    let txaux = finalized.make_txaux().unwrap_or_else(|e| term.fail_with(e));

    writeln!(term, "sending transaction {}", style!(txaux.tx.id()));

    for np in blockchain.peers() {
        if ! np.is_native() { continue; }

        let peer = super::super::blockchain::peer::Peer::prepare(&blockchain, np.name().to_owned());

        peer.connect(&mut term).unwrap().send_txaux(txaux.clone())
    }
}

pub fn sign( mut term: Term
           , root_dir: PathBuf
           , id_str: &str
           )
{
    let mut signatures = Vec::new();
    let mut staging = load_staging(&mut term, root_dir.clone(), id_str);

    let mut wallets = BTreeMap::new();
    for (name, wallet) in Wallets::load(root_dir.clone()).unwrap() {
        let state = wallet::utils::create_wallet_state_from_logs(&mut term, &wallet, root_dir.clone(), wallet::state::lookup::accum::Accum::default());
        wallets.insert(name, (wallet, state));
    }

    let (finalized, changes) = staging.transaction().mk_finalized().unwrap_or_else(|e| term.fail_with(e));
    let txaux = finalized.make_txaux().unwrap_or_else(|e| term.fail_with(e));
    let txid = txaux.tx.id();
    let protocol_magic = staging.protocol_magic;

    // TODO: ignore already signed inputs
    for input in staging.transaction().inputs() {
        let txin = input.extract_txin();
        let mut signature = None;
        for (name, (wallet, state)) in wallets.iter() {
            if let Some(utxo) = state.utxos.get(&txin) {
                term.info(
                    &format!(
                        "signing input {}.{} ({})\n",
                        style!(input.transaction_id),
                        style!(input.index_in_transaction),
                        style!(name)
                    )
                ).unwrap();

                signature = Some(wallet::utils::wallet_sign_tx(
                    &mut term, wallet, protocol_magic, &txid, &utxo.credited_addressing
                ));
            }
        }

        if let Some(signature) = signature {
            signatures.push(signature);
        } else {
            panic!("cannot sign input {:#?}", input)
        }
    }

    for signature in signatures {
        staging.add_signature(signature).unwrap();
    }
}

pub fn status( mut term: Term
             , root_dir: PathBuf
             , id_str: &str
             )
{
    let staging = load_staging(&mut term, root_dir, id_str);

    let trans = staging.transaction();
    let inputs = trans.inputs();
    let input_total = sum_coins(inputs.into_iter().map(|x| x.expected_value)).unwrap();
    let (finalized, changes) = staging.transaction().mk_txbuilder().unwrap_or_else(|e| term.fail_with(e));
    let tx = finalized.make_tx().unwrap_or_else(|e| term.fail_with(e));
    let output_total = tx.get_output_total().unwrap();
    let difference = {
        let i : u64 = input_total.into();
        let o : u64 = output_total.into();
        (i as i64) - (o as i64)
    };

    let fee_alg = LinearFee::default();
    let fake_witnesses : Vec<TxInWitness> = iter::repeat(TxInWitness::fake()).take(inputs.len()).collect();
    let fee = fee_alg.calculate_for_txaux_component(&tx, &fake_witnesses).unwrap();

    let txbytes_length = tx::txaux_serialize_size(&tx, &fake_witnesses);

    writeln!(term, "input-total: {}", input_total);
    writeln!(term, "output-total: {}", output_total);
    writeln!(term, "actual fee: {}", difference);
    writeln!(term, "fee: {}", fee.to_coin());
    writeln!(term, "tx-bytes: {}", txbytes_length);

    writeln!(term, "inputs:");
    for input in tx.inputs.iter() {
        writeln!(term, "  {}.{}", style!(input.id), style!(input.index));
    }
    writeln!(term, "outputs:");
    for output in tx.outputs.iter() {
        writeln!(term, "  {} {}", style!(&output.address), style!(output.value));
    }
}

pub fn add_input( mut term: Term
                , root_dir: PathBuf
                , id_str: &str
                , input: Option<(TxId, u32, Option<Coin>)>
                )
{
    let mut staging = load_staging(&mut term, root_dir.clone(), id_str);

    let input = if let Some(input) = input {
        match input.2 {
            None => {
                find_input_in_all_utxos(&mut term, root_dir.clone(), input.0, input.1)
            },
            Some(v) => {
                core::Input {
                    transaction_id: input.0,
                    index_in_transaction: input.1,
                    expected_value: v,
                }
            },
        }
    } else {
        // TODO, implement interactive mode
        unimplemented!()
    };

    staging.add_input(input).unwrap_or_else(|e| term.fail_with(e))
}

pub fn add_output( mut term: Term
                 , root_dir: PathBuf
                 , id_str: &str
                 , output: Option<(ExtendedAddr, Coin)>
                 )
{
    let mut staging = load_staging(&mut term, root_dir, id_str);

    let output = if let Some(output) = output {
        core::Output {
            address: output.0,
            amount:  output.1
        }
    } else {
        // TODO, implement interactive mode
        unimplemented!()
    };

    staging.add_output(output).unwrap_or_else(|e| term.fail_with(e))
}

pub fn add_change( mut term: Term
                 , root_dir: PathBuf
                 , id_str: &str
                 , change: ExtendedAddr
                 )
{
    let mut staging = load_staging(&mut term, root_dir, id_str);

    staging.add_change(change.into()).unwrap_or_else(|e| term.fail_with(e))
}

pub fn remove_input( mut term: Term
                   , root_dir: PathBuf
                   , id_str: &str
                   , input: Option<(TxId, u32)>
                   )
{
    let mut staging = load_staging(&mut term, root_dir, id_str);

    let txin = if let Some(input) = input {
        TxoPointer {
            id: input.0,
            index: input.1
        }
    } else {
        // TODO, implement interactive mode
        unimplemented!()
    };

    staging.remove_input(txin).unwrap_or_else(|e| term.fail_with(e))
}

pub fn remove_output( mut term: Term
                    , root_dir: PathBuf
                    , id_str: &str
                    , address: Option<ExtendedAddr>
                    )
{
    let mut staging = load_staging(&mut term, root_dir, id_str);

    if let Some(addr) = address {
        staging.remove_outputs_for(&addr).unwrap_or_else(|e| term.fail_with(e))
    } else {
        // TODO, implement interactive mode
        unimplemented!()
    };
}

pub fn remove_change( mut term: Term
                    , root_dir: PathBuf
                    , id_str: &str
                    , change: ExtendedAddr
                    )
{
    let mut staging = load_staging(&mut term, root_dir, id_str);

    staging.remove_change(change).unwrap_or_else(|e| term.fail_with(e))
}

pub fn finalize( mut term: Term
               , root_dir: PathBuf
               , id_str: &str
               )
{
    let mut staging = load_staging(&mut term, root_dir, id_str);

    staging.finalize().unwrap_or_else(|e| term.fail_with(e))
}

pub fn export( mut term: Term
             , root_dir: PathBuf
             , id_str: &str
             , export_file: Option<&str>
             )
{
    let staging = load_staging(&mut term, root_dir, id_str);

    let export = staging.export();

    if let Some(export_file) = export_file {
        let mut file = ::std::fs::OpenOptions::new().create(true).write(true).open(export_file).unwrap();
        ::serde_yaml::to_writer(&mut file, &export).unwrap();
    } else {
        ::serde_yaml::to_writer(&mut term, &export).unwrap();
    }
}

pub fn import( mut term: Term
             , root_dir: PathBuf
             , import_file: Option<&str>
             )
{
    let import = if let Some(import_file) = import_file {
        let mut file = ::std::fs::OpenOptions::new().read(true).open(import_file).unwrap();
        ::serde_yaml::from_reader(&mut file).unwrap()
    } else {
        let mut stdin = ::std::io::stdin();
        ::serde_yaml::from_reader(&mut stdin).unwrap()
    };

    let staging = StagingTransaction::import(root_dir, import).unwrap_or_else(|e| term.fail_with(e));
    writeln!(&mut term, "Staging transaction `{}' successfully imported",
        style!(staging.id())
    );
}

pub fn input_select( mut term: Term
                   , root_dir: PathBuf
                   , id_str: &str
                   , wallets: Vec<WalletName>
                   )
{
    use ::cardano::{fee::{self}, input_selection::{SelectionAlgorithm, SelectionPolicy}, txutils};

    let alg = fee::LinearFee::default();
    let selection_policy = SelectionPolicy::default();

    let mut staging = load_staging(&mut term, root_dir.clone(), id_str);

    if staging.is_finalized() {
        term.error("Cannot select inputs to a finalized staging transaction").unwrap();
        ::std::process::exit(1);
    }

    if ! staging.transaction().has_change() {
        term.error("cannot select inputs if no change").unwrap();
        ::std::process::exit(1);
    }

    let change_address = staging.transaction().changes()[0].address.clone();
    let output_policy = txutils::OutputPolicy::One(change_address.clone());

    let outputs = staging.transaction().outputs().iter().map(|output| {
        output.into()
    }).collect::<Vec<_>>();
    let inputs = list_input_inputs(&mut term, root_dir.clone(), wallets);

    let result = alg.compute(
        selection_policy,
        inputs.iter(),
        outputs.iter(),
        &output_policy
    );
    let (_, selected_inputs, change) = match result {
        Err(err) => { panic!("error {:#?}", err) },
        Ok(v) => v
    };

    for input in selected_inputs {
        staging.add_input(core::Input {
            transaction_id: input.ptr.id,
            index_in_transaction: input.ptr.index,
            expected_value: input.value.value
        }).unwrap_or_else(|e| term.fail_with(e));
    }
}

/// helper function to load a staging file
fn load_staging(term: &mut Term, root_dir: PathBuf, id_str: &str) -> StagingTransaction {
    let id = match id_str.parse::<StagingId>() {
        Err(err) => {
            debug!("cannot parse staging id: {:?}", err);
            term.error("Invalid StagingId\n").unwrap();
            ::std::process::exit(1);
        },
        Ok(id) => id
    };

    match StagingTransaction::read_from_file(root_dir, id) {
        Err(err) => {
            error!("Error while loading a staging transaction: {:?}", err);
            term.error("Cannot load the staging transaction\n").unwrap();
            ::std::process::exit(1);
        },
        Ok(st) => st
    }
}

// ----------------------------------- helpers ---------------------------------

fn find_input_in_all_utxos(term: &mut Term, root_dir: PathBuf, txid: TxId, index: u32) -> core::Input {
    let txin = TxoPointer { id: txid, index: index };
    for (_, wallet) in Wallets::load(root_dir.clone()).unwrap() {
        let state = wallet::utils::create_wallet_state_from_logs(term, &wallet, root_dir.clone(), wallet::state::lookup::accum::Accum::default());

        if let Some(utxo) = state.utxos.get(&txin) {
            let txin = utxo.extract_txin();
            return core::Input {
                transaction_id: txin.id,
                index_in_transaction: txin.index,
                expected_value: utxo.credited_value,
            };
        }
    }

    term.error(&format!("No input found")).unwrap();
    ::std::process::exit(1);
}

fn list_input_inputs(term: &mut Term, root_dir: PathBuf, wallets: Vec<WalletName>) -> Vec<::cardano::txutils::Input<ExtendedAddr>> {
    let mut inputs = Vec::new();
    for wallet in wallets {
        let wallet = Wallet::load(root_dir.clone(), wallet);
        let state = wallet::utils::create_wallet_state_from_logs(term, &wallet, root_dir.clone(), wallet::state::lookup::accum::Accum::default());

        inputs.extend(state.utxos.iter().map(|(_, utxo)| {
            let txin = utxo.extract_txin();
            let txout = utxo.extract_txout();
            ::cardano::txutils::Input::new(
                txin,
                txout,
                utxo.credited_address.clone()
            )
        }))
    }

    inputs
}
