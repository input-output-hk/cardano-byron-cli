use super::core;
use super::super::blockchain;
use cardano::{
    self,
    coin,
};
use storage_units;

use std::{io::{self, Write}, fmt, error};

#[derive(Debug)]
pub enum Error {
    IoError(io::Error),
    InvalidStagingId(core::staging_id::ParseStagingIdError),
    CannotLoadBlockchain(blockchain::Error),
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
impl From<blockchain::Error> for Error {
    fn from(e: blockchain::Error) -> Self { Error::CannotLoadBlockchain(e) }
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::Error::*;
        match self {
            IoError(_)                                 => write!(f, "I/O Error"),
            InvalidStagingId(_)                        => write!(f, "Invalid Staging ID"),
            CannotLoadBlockchain(_)                    => write!(f, "Cannot load the blockchain"),
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
            CannotLoadBlockchain(ref err)                    => Some(err),
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
