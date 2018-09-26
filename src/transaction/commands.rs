use std::{path::PathBuf, io::{self, Write}, iter, collections::BTreeMap, fmt, error};
use utils::term::{Term, style::{Style}};
use super::core::{self, StagingId, StagingTransaction};
use super::super::blockchain::{Blockchain, BlockchainName};
use super::super::wallet::{Wallets, Wallet, self, WalletName};
use cardano::{self, tx::{self, Tx, TxId, TxoPointer, TxInWitness}, coin::{self, Coin, sum_coins}, address::{ExtendedAddr}, fee::{LinearFee, FeeAlgorithm}};
use storage_units;

#[derive(Debug)]
pub enum Error {
    IoError(::std::io::Error),
    InvalidStagingId(core::staging_id::ParseStagingIdError),
    CannotLoadStagingTransaction(core::staging_transaction::StagingTransactionParseError),

    CannotCreateNewTransaction(storage_units::append::Error),
    CannotDestroyTransaction(storage_units::append::Error),
    CannotSendTransactionNotFinalized(core::transaction::Error),
    CannotSendTransactionInvalidTxAux(cardano::txbuild::Error),
    CannotSendTransactionNotSent,
    CannotSignTransactionNotFinalized(core::transaction::Error),
    CannotSignTransactionInvalidTxAux(cardano::txbuild::Error),
    CannotSignTransactionCannotAddSignature(core::staging_transaction::StagingUpdateError),
    CannotReportStatusInvalidInputTotal(coin::Error),
    CannotReportStatusInvalidOutputTotal(coin::Error),
    CannotReportStatusInvalidTxBuilder(core::transaction::Error),
    CannotReportStatusInvalidTx(cardano::txbuild::Error),
    CannotReportStatusInvalidFee(cardano::fee::Error),
    CannotAddInput(core::staging_transaction::StagingUpdateError),
    CannotFindInputsInAllLocalUtxos,
    CannotAddOutput(core::staging_transaction::StagingUpdateError),
    CannotAddChange(core::staging_transaction::StagingUpdateError),
    CannotRemoveInput(core::staging_transaction::StagingUpdateError),
    CannotRemoveOutput(core::staging_transaction::StagingUpdateError),
    CannotRemoveChange(core::staging_transaction::StagingUpdateError),
    CannotFinalize(core::staging_transaction::StagingUpdateError),
    CannotExportToFileCannotOpenOutFile(io::Error),
    CannotExportToFile(::serde_yaml::Error),
    CannotExportToStdout(::serde_yaml::Error),
    CannotImportFromFileCannotOpenInputFile(io::Error),
    CannotImportFromFile(::serde_yaml::Error),
    CannotImportFromStdin(::serde_yaml::Error),
    CannotImportStaging(core::staging_transaction::StagingUpdateError),

    CannotInputSelectNoChangeOption,
    CannotInputSelectSelectionFailed(cardano::input_selection::Error),
    CannotInputSelectCannotAddInput(core::staging_transaction::StagingUpdateError),
}
impl From<::std::io::Error> for Error {
    fn from(e: ::std::io::Error) -> Self { Error::IoError(e) }
}
impl From<core::staging_id::ParseStagingIdError> for Error {
    fn from(e: core::staging_id::ParseStagingIdError) -> Self { Error::InvalidStagingId(e) }
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::Error::*;
        match self {
            IoError(_)                                 => write!(f, "I/O Error"),
            InvalidStagingId(_)                        => write!(f, "Invalid Staging ID"),
            CannotLoadStagingTransaction(_)            => write!(f, "Cannot load the staging transaction"),
            CannotCreateNewTransaction(_)              => write!(f, "Cannot create a new Staging Transaction"),
            CannotDestroyTransaction(_)                => write!(f, "Cannot destroy the Staging Transaction"),
            CannotSendTransactionNotFinalized(_)       => write!(f, "Cannot send transaction, finalize it first"),
            CannotSendTransactionInvalidTxAux(_)       => write!(f, "Cannot send transaction"),
            CannotSendTransactionNotSent               => write!(f, "Cannot send transaction to any blockchain peers"),
            CannotSignTransactionNotFinalized(_)       => write!(f, "Cannot sign transaction, finalize it first"),
            CannotSignTransactionInvalidTxAux(_)       => write!(f, "Cannot sign transaction"),
            CannotSignTransactionCannotAddSignature(_) => write!(f, "Cannot add signature to the transaction"),
            CannotReportStatusInvalidInputTotal(_)     => write!(f, "Input total of the transaction is invalid"),
            CannotReportStatusInvalidOutputTotal(_)    => write!(f, "Output total of the transaction is invalid"),
            CannotReportStatusInvalidTxBuilder(_)      => write!(f, "Cannot gather the transaction status"),
            CannotReportStatusInvalidTx(_)             => write!(f, "The transaction is not valid"),
            CannotReportStatusInvalidFee(_)            => write!(f, "Fee computation returned an error"),
            CannotAddInput(_)                          => write!(f, "Cannot add input to staging transaction"),
            CannotFindInputsInAllLocalUtxos            => write!(f, "Cannot find inputs within the local UTxOs"),
            CannotAddOutput(_)                         => write!(f, "Cannot add output to the staging transaction"),
            CannotAddChange(_)                         => write!(f, "Cannot add change to the staging transaction"),
            CannotRemoveInput(_)                       => write!(f, "Cannot remove input from the staging transaction"),
            CannotRemoveOutput(_)                      => write!(f, "Cannot remove output from the staging transaction"),
            CannotRemoveChange(_)                      => write!(f, "Cannot remove change from the staging transaction"),
            CannotFinalize(_)                          => write!(f, "Cannot finalize the staging transaction"),
            CannotExportToFileCannotOpenOutFile(_)     => write!(f, "Cannot export the staging transaction: cannot open output file"),
            CannotExportToFile(_)                      => write!(f, "Cannot export the staging transaction to the output file"),
            CannotExportToStdout(_)                    => write!(f, "Cannot export the staging transaction to the standard output"),
            CannotImportFromFileCannotOpenInputFile(_) => write!(f, "Cannot import the staging transaction: cannot open input file"),
            CannotImportFromFile(_)                    => write!(f, "Cannot import the staging transaction from the input file"),
            CannotImportFromStdin(_)                   => write!(f, "Cannot import the staging transaction from the standard input"),
            CannotImportStaging(_)                     => write!(f, "Cannot import the staging transaction: invalid or corrupted"),
            CannotInputSelectNoChangeOption            => write!(f, "Add change before trying to run the input selection algorithm"),
            CannotInputSelectSelectionFailed(_)        => write!(f, "Input selection algorithm failed to run"),
            CannotInputSelectCannotAddInput(_)         => write!(f, "Cannot add input to the staging transaction"),
        }
    }
}
impl error::Error for Error {
    fn cause(&self) -> Option<& error::Error> {
        use self::Error::*;
        match self {
            IoError(ref err)                                 => Some(err),
            InvalidStagingId(ref err)                        => Some(err),
            CannotLoadStagingTransaction(ref err)            => Some(err),
            CannotCreateNewTransaction(ref err)              => Some(err),
            CannotDestroyTransaction(ref err)                => Some(err),
            CannotSendTransactionNotFinalized(ref err)       => Some(err),
            CannotSendTransactionInvalidTxAux(ref err)       => Some(err),
            CannotSendTransactionNotSent               => None,
            CannotSignTransactionNotFinalized(ref err)       => Some(err),
            CannotSignTransactionInvalidTxAux(ref err)       => Some(err),
            CannotSignTransactionCannotAddSignature(ref err) => Some(err),
            CannotReportStatusInvalidInputTotal(ref err)     => Some(err),
            CannotReportStatusInvalidOutputTotal(ref err)    => Some(err),
            CannotReportStatusInvalidTxBuilder(ref err)      => Some(err),
            CannotReportStatusInvalidTx(ref err)             => Some(err),
            CannotReportStatusInvalidFee(ref err)            => Some(err),
            CannotAddInput(ref err)                          => Some(err),
            CannotFindInputsInAllLocalUtxos            => None,
            CannotAddOutput(ref err)                         => Some(err),
            CannotAddChange(ref err)                         => Some(err),
            CannotRemoveInput(ref err)                       => Some(err),
            CannotRemoveOutput(ref err)                      => Some(err),
            CannotRemoveChange(ref err)                      => Some(err),
            CannotFinalize(ref err)                          => Some(err),
            CannotExportToFileCannotOpenOutFile(ref err)     => Some(err),
            CannotExportToFile(ref err)                      => Some(err),
            CannotExportToStdout(ref err)                    => Some(err),
            CannotImportFromFileCannotOpenInputFile(ref err) => Some(err),
            CannotImportFromFile(ref err)                    => Some(err),
            CannotImportFromStdin(ref err)                   => Some(err),
            CannotImportStaging(ref err)                     => Some(err),
            CannotInputSelectNoChangeOption            => None,
            CannotInputSelectSelectionFailed(ref err)        => Some(err),
            CannotInputSelectCannotAddInput(ref err)         => Some(err),
        }
    }
}

/// function to create a new empty transaction
pub fn new( term: &mut Term
          , root_dir: PathBuf
          , blockchain: BlockchainName
          )
    -> Result<(), Error>
{
    let blockchain = Blockchain::load(root_dir.clone(), blockchain);

    let staging = StagingTransaction::new(root_dir, blockchain.config.protocol_magic)
        .map_err(Error::CannotCreateNewTransaction)?;

    writeln!(term, "{}", style!(staging.id()))?;

    Ok(())
}

pub fn list( term: &mut Term
           , root_dir: PathBuf
           )
    -> Result<(), Error>
{
    let transactions_dir = core::config::transaction_directory(root_dir.clone());

    for entry in ::std::fs::read_dir(transactions_dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            term.warn(&format!("unexpected directory in transaction directory: {:?}", entry.path()))?;
            continue;
        }
        let name = entry.file_name().into_string().unwrap_or_else(|err| {
            panic!("invalid utf8... {:?}", err)
        });

        let staging = load_staging(root_dir.clone(), name.as_str())?;

        writeln!(term, "{}", style!(staging.id())).unwrap();
    }

    Ok(())
}

pub fn destroy( _term: &mut Term
              , root_dir: PathBuf
              , id_str: &str
              )
    -> Result<(), Error>
{
    let staging = load_staging(root_dir, id_str)?;

    staging.destroy().map_err(Error::CannotDestroyTransaction)
}

/// function to create a new empty transaction
pub fn send( term: &mut Term
           , root_dir: PathBuf
           , id_str: &str
           , blockchain: BlockchainName
           )
    -> Result<(), Error>
{
    let blockchain = Blockchain::load(root_dir.clone(), blockchain);
    let staging = load_staging(root_dir.clone(), id_str)?;

    let (finalized, changes) = staging.transaction().mk_finalized()
        .map_err(Error::CannotSendTransactionNotFinalized)?;
    let txaux = finalized.make_txaux()
        .map_err(Error::CannotSendTransactionInvalidTxAux)?;

    writeln!(term, "sending transaction {}", style!(txaux.tx.id()))?;

    let mut sent = false;
    for np in blockchain.peers() {
        if ! np.is_native() { continue; }

        let peer = super::super::blockchain::peer::Peer::prepare(&blockchain, np.name().to_owned());

        sent = sent || peer.connect(term).unwrap().send_txaux(txaux.clone());
    }

    if sent { Ok(()) } else {
        Err(Error::CannotSendTransactionNotSent)
    }
}

pub fn sign( term: &mut Term
           , root_dir: PathBuf
           , id_str: &str
           )
    -> Result<(), Error>
{
    let mut signatures = Vec::new();

    let mut wallets = BTreeMap::new();
    for (name, wallet) in Wallets::load(root_dir.clone()).unwrap() {
        let state = wallet::utils::create_wallet_state_from_logs(term, &wallet, root_dir.clone(), wallet::state::lookup::accum::Accum::default());
        wallets.insert(name, (wallet, state));
    }

    let mut staging = load_staging(root_dir.clone(), id_str)?;
    let (finalized, changes) = staging.transaction().mk_finalized()
        .map_err(Error::CannotSignTransactionNotFinalized)?;
    let tx = staging.transaction().mk_txbuilder()
        .map_err(Error::CannotSignTransactionNotFinalized)?
        .0.make_tx()
        .map_err(Error::CannotSignTransactionInvalidTxAux)?;
    let txid = tx.id();
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
                    term, wallet, protocol_magic, &txid, &utxo.credited_addressing
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
        staging.add_signature(signature)
            .map_err(Error::CannotSignTransactionCannotAddSignature)?;
    }
    Ok(())
}

pub fn status( term: &mut Term
             , root_dir: PathBuf
             , id_str: &str
             )
    -> Result<(), Error>
{
    let staging = load_staging(root_dir, id_str)?;

    let trans = staging.transaction();
    let inputs = trans.inputs();
    let input_total = sum_coins(inputs.into_iter().map(|x| x.expected_value))
        .map_err(Error::CannotReportStatusInvalidInputTotal)?;
    let (builder, changes) = staging.transaction().mk_txbuilder()
        .map_err(Error::CannotReportStatusInvalidTxBuilder)?;
    let tx = builder.make_tx()
        .unwrap_or_else(|_| Tx::new());
    let output_total = tx.get_output_total()
        .map_err(Error::CannotReportStatusInvalidOutputTotal)?;
    let difference = {
        let i : u64 = input_total.into();
        let o : u64 = output_total.into();
        (i as i64) - (o as i64)
    };

    let fee_alg = LinearFee::default();
    let fake_witnesses : Vec<TxInWitness> = iter::repeat(TxInWitness::fake()).take(inputs.len()).collect();
    let fee = fee_alg.calculate_for_txaux_component(&tx, &fake_witnesses)
        .map_err(Error::CannotReportStatusInvalidFee)?;

    let txbytes_length = tx::txaux_serialize_size(&tx, &fake_witnesses);

    writeln!(term, "input-total: {}", input_total)?;
    writeln!(term, "output-total: {}", output_total)?;
    writeln!(term, "actual fee: {}.{}", difference / 1000000, difference % 1000000)?;
    writeln!(term, "fee: {}", fee.to_coin())?;
    writeln!(term, "tx-bytes: {}", txbytes_length)?;

    writeln!(term, "inputs:")?;
    for input in tx.inputs.iter() {
        writeln!(term, "  {}.{}", style!(input.id), style!(input.index))?;
    }
    writeln!(term, "outputs:")?;
    for output in tx.outputs.iter() {
        writeln!(term, "  {} {}", style!(&output.address), style!(output.value))?;
    }

    Ok(())
}

pub fn add_input( term: &mut Term
                , root_dir: PathBuf
                , id_str: &str
                , input: Option<(TxId, u32, Option<Coin>)>
                )
    -> Result<(), Error>
{
    let mut staging = load_staging(root_dir.clone(), id_str)?;

    let input = if let Some(input) = input {
        match input.2 {
            None => {
                find_input_in_all_utxos(term, root_dir.clone(), input.0, input.1)?
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

    staging.add_input(input).map_err(Error::CannotAddInput)
}

pub fn add_output( _term: &mut Term
                 , root_dir: PathBuf
                 , id_str: &str
                 , output: Option<(ExtendedAddr, Coin)>
                 )
    -> Result<(), Error>
{
    let mut staging = load_staging(root_dir, id_str)?;

    let output = if let Some(output) = output {
        core::Output {
            address: output.0,
            amount:  output.1
        }
    } else {
        // TODO, implement interactive mode
        unimplemented!()
    };

    staging.add_output(output).map_err(Error::CannotAddOutput)
}

pub fn add_change( _term: &mut Term
                 , root_dir: PathBuf
                 , id_str: &str
                 , change: ExtendedAddr
                 )
    -> Result<(), Error>
{
    let mut staging = load_staging(root_dir, id_str)?;

    staging.add_change(change.into()).map_err(Error::CannotAddChange)
}

pub fn remove_input( _term: &mut Term
                   , root_dir: PathBuf
                   , id_str: &str
                   , input: Option<(TxId, u32)>
                   )
    -> Result<(), Error>
{
    let mut staging = load_staging(root_dir, id_str)?;

    let txin = if let Some(input) = input {
        TxoPointer {
            id: input.0,
            index: input.1
        }
    } else {
        // TODO, implement interactive mode
        unimplemented!()
    };

    staging.remove_input(txin).map_err(Error::CannotRemoveInput)
}

pub fn remove_output( _term: &mut Term
                    , root_dir: PathBuf
                    , id_str: &str
                    , address: Option<ExtendedAddr>
                    )
    -> Result<(), Error>
{
    let mut staging = load_staging(root_dir, id_str)?;

    if let Some(addr) = address {
        staging.remove_outputs_for(&addr).map_err(Error::CannotRemoveOutput)
    } else {
        // TODO, implement interactive mode
        unimplemented!()
    }
}

pub fn remove_change( _term: &mut Term
                    , root_dir: PathBuf
                    , id_str: &str
                    , change: ExtendedAddr
                    )
    -> Result<(), Error>
{
    let mut staging = load_staging(root_dir, id_str)?;

    staging.remove_change(change).map_err(Error::CannotRemoveChange)
}

pub fn finalize( _term: &mut Term
               , root_dir: PathBuf
               , id_str: &str
               )
    -> Result<(), Error>
{
    let mut staging = load_staging(root_dir, id_str)?;

    staging.finalize().map_err(Error::CannotFinalize)
}

pub fn export( term: &mut Term
             , root_dir: PathBuf
             , id_str: &str
             , export_file: Option<&str>
             )
    -> Result<(), Error>
{
    let staging = load_staging(root_dir, id_str)?;

    let export = staging.export();

    if let Some(export_file) = export_file {
        let mut file = ::std::fs::OpenOptions::new().create(true).write(true).open(export_file)
            .map_err(Error::CannotExportToFileCannotOpenOutFile)?;
        ::serde_yaml::to_writer(&mut file, &export)
            .map_err(Error::CannotExportToFile)
    } else {
        ::serde_yaml::to_writer(term, &export)
            .map_err(Error::CannotExportToStdout)
    }
}

pub fn import( term: &mut Term
             , root_dir: PathBuf
             , import_file: Option<&str>
             )
    -> Result<(), Error>
{
    let import = if let Some(import_file) = import_file {
        let mut file = ::std::fs::OpenOptions::new().read(true).open(import_file)
            .map_err(Error::CannotImportFromFileCannotOpenInputFile)?;
        ::serde_yaml::from_reader(&mut file)
            .map_err(Error::CannotImportFromFile)?
    } else {
        let mut stdin = ::std::io::stdin();
        ::serde_yaml::from_reader(&mut stdin)
            .map_err(Error::CannotImportFromStdin)?
    };

    let staging = StagingTransaction::import(root_dir, import)
        .map_err(Error::CannotImportStaging)?;
    writeln!(term, "Staging transaction `{}' successfully imported", style!(staging.id()))?;
    Ok(())
}

pub fn input_select( term: &mut Term
                   , root_dir: PathBuf
                   , id_str: &str
                   , wallets: Vec<WalletName>
                   )
    -> Result<(), Error>
{
    use ::cardano::{fee::{self}, input_selection::{SelectionAlgorithm, SelectionPolicy}, txutils};

    let alg = fee::LinearFee::default();
    let selection_policy = SelectionPolicy::default();

    let mut staging = load_staging(root_dir.clone(), id_str)?;

    if ! staging.transaction().has_change() {
        return Err(Error::CannotInputSelectNoChangeOption);
    }

    let change_address = staging.transaction().changes()[0].address.clone();
    let output_policy = txutils::OutputPolicy::One(change_address.clone());

    let outputs = staging.transaction().outputs().iter().map(|output| {
        output.into()
    }).collect::<Vec<_>>();
    let inputs = list_input_inputs(term, root_dir.clone(), wallets);

    let (_, selected_inputs, change) = alg.compute(
        selection_policy,
        inputs.iter(),
        outputs.iter(),
        &output_policy
    ).map_err(Error::CannotInputSelectSelectionFailed)?;

    for input in selected_inputs {
        staging.add_input(core::Input {
            transaction_id: input.ptr.id,
            index_in_transaction: input.ptr.index,
            expected_value: input.value.value
        }).map_err(Error::CannotInputSelectCannotAddInput)?;
    }
    Ok(())
}

/// helper function to load a staging file
fn load_staging(root_dir: PathBuf, id_str: &str) -> Result<StagingTransaction, Error> {
    let id = id_str.parse::<StagingId>().map_err(Error::InvalidStagingId)?;

    StagingTransaction::read_from_file(root_dir, id).map_err(Error::CannotLoadStagingTransaction)
}

// ----------------------------------- helpers ---------------------------------

fn find_input_in_all_utxos(term: &mut Term, root_dir: PathBuf, txid: TxId, index: u32)
    -> Result<core::Input, Error>
{
    let txin = TxoPointer { id: txid, index: index };
    for (_, wallet) in Wallets::load(root_dir.clone()).unwrap() {
        let state = wallet::utils::create_wallet_state_from_logs(term, &wallet, root_dir.clone(), wallet::state::lookup::accum::Accum::default());

        if let Some(utxo) = state.utxos.get(&txin) {
            let txin = utxo.extract_txin();
            return Ok(core::Input {
                transaction_id: txin.id,
                index_in_transaction: txin.index,
                expected_value: utxo.credited_value,
            });
        }
    }

    Err(Error::CannotFindInputsInAllLocalUtxos)
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
