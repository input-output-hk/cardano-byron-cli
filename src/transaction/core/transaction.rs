use super::{Operation, Input, Output, Change};
use cardano::{tx::{TxoPointer, TxOut, TxWitness, TxInWitness}, address::{ExtendedAddr}};
use cardano::{txbuild::{self, TxBuilder, TxFinalized}, fee::{LinearFee}, txutils::{OutputPolicy}};
use std::{fmt, error};

#[derive(Debug)]
pub enum Error {
    /// happened user attempts to finalize a transaction already
    /// in a finalizing state
    CannotFinalizeAFinalizedTransaction,

    CannotAddWitnessesToAnOpenedTransaction,

    CannotAddMoreWitnessesThanInputs,

    CannotAddInputsToAFinalizedTransaction,

    CannotAddOutputToAFinalizedTransaction,

    CannotAddChangeToAFinalizedTransaction,

    TransactionNotFinalized,

    /// when input is duplicated in the transaction
    DoubleSpend,

    CannotRemoveInputInputNotFound,
    CannotRemoveOutput,
    CannotRemoveChangeChangeNotFound,

    /// TODO: this is temporary only until we can support selection
    /// policy with multiple change addresses.
    ///
    /// In the mean time we need to ask users to remove the previous output
    /// or to keep the current one.
    MoreThanOneChangeAddressIsNotSupportedYet,

    ErrorWhenApplyingOutputPolicy(txbuild::Error),

    CannotBuildTxFromBuilder(txbuild::Error),
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::CannotFinalizeAFinalizedTransaction => write!(f, "Transaction is already in a finalized state"),
            Error::CannotAddWitnessesToAnOpenedTransaction => write!(f, "Transaction is not finalized, finalize the transaction before adding witnesses"),
            Error::CannotAddMoreWitnessesThanInputs => write!(f, "There is already enough witness for the transaction, cannot add more witnesses than inputs."),
            Error::CannotAddInputsToAFinalizedTransaction => write!(f, "Transaction is in a finalized state, cannot add more inputs"),
            Error::CannotAddOutputToAFinalizedTransaction => write!(f, "Transaction is in a finalized state, cannot add more outputs"),
            Error::CannotAddChangeToAFinalizedTransaction => write!(f, "Transaction is in a finalized state, cannot add more change addresses"),
            Error::TransactionNotFinalized => write!(f, "Transaction is not finalized, finalize it first"),
            Error::DoubleSpend => write!(f, "Input already used in the transaction"),
            Error::CannotRemoveInputInputNotFound => write!(f, "Cannot remove input, input not found"),
            Error::CannotRemoveOutput => write!(f, "Cannot remove output, output not found"),
            Error::CannotRemoveChangeChangeNotFound => write!(f, "Cannot remove change, change address not found"),
            Error::MoreThanOneChangeAddressIsNotSupportedYet => write!(f, "Cannot add more than one output address for now, this feature is not yet supported"),
            Error::ErrorWhenApplyingOutputPolicy(_) => write!(f, "Error when applying the output policy utilising the changes"),
            Error::CannotBuildTxFromBuilder(_) => write!(f, "Error when constructing the Tx, invalid data."),
        }
    }
}
impl error::Error for Error {
    fn cause(&self) -> Option<& error::Error> {
        match self {
            Error::CannotFinalizeAFinalizedTransaction => None,
            Error::CannotAddWitnessesToAnOpenedTransaction => None,
            Error::CannotAddMoreWitnessesThanInputs => None,
            Error::CannotAddInputsToAFinalizedTransaction => None,
            Error::CannotAddOutputToAFinalizedTransaction => None,
            Error::CannotAddChangeToAFinalizedTransaction => None,
            Error::TransactionNotFinalized => None,
            Error::DoubleSpend => None,
            Error::CannotRemoveInputInputNotFound => None,
            Error::CannotRemoveOutput => None,
            Error::CannotRemoveChangeChangeNotFound => None,
            Error::MoreThanOneChangeAddressIsNotSupportedYet => None,
            Error::ErrorWhenApplyingOutputPolicy(ref err) => Some(err),
            Error::CannotBuildTxFromBuilder(ref err) => Some(err),
        }
    }
}

type Result<T> = ::std::result::Result<T, Error>;


/// describe a transaction in its most reduce representation
///
/// Transaction are not meant to be edited from this representation
/// as this is a read only object.
///
/// There is 2 way to construct a transaction:
///
/// 1. by creating an empty transaction and updating it with operations;
/// 2. by collecting it from an iterator over `Operation` (see `FromIterator` trait);
///
/// Keeping private the transaction will allow us to control the state of the transaction
/// and to guarantee some levels of integrity (preventing errors).
///
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub inputs:    Vec<Input>,
    pub outputs:   Vec<Output>,
    pub changes:   Vec<Change>,
    pub witnesses: TxWitness,
    pub finalized: bool
}
impl Transaction {
    /// create an empty transaction
    pub fn new() -> Self {
        Transaction {
            inputs: Vec::new(),
            outputs: Vec::new(),
            changes: Vec::new(),
            witnesses: TxWitness::new(),
            finalized: false,
        }
    }

    pub fn mk_txbuilder(&self) -> Result<(TxBuilder, Vec<TxOut>)> {
        let mut builder = TxBuilder::new();
        for input in self.inputs.iter() {
            let ptr = TxoPointer { id: input.transaction_id, index: input.index_in_transaction };
            let value = input.expected_value;
            builder.add_input(&ptr, value);
        }
        for output in self.outputs.iter() {
            let out = TxOut { address: output.address.clone(), value: output.amount };
            builder.add_output_value(&out);
        }
        let changes_used = if self.changes.len() == 1 {
            let fee_algorithm = LinearFee::default();
            builder.add_output_policy(&fee_algorithm, &OutputPolicy::One(self.changes[0].address.clone()))
                .map_err(Error::ErrorWhenApplyingOutputPolicy)?
        } else { Vec::new() };

        Ok((builder, changes_used))
    }

    pub fn mk_finalized(&self) -> Result<(TxFinalized, Vec<TxOut>)> {
        let (builder, changes_used) = self.mk_txbuilder()?;

        if ! self.is_finalized() {
            return Err(Error::TransactionNotFinalized);
        }
        let tx = builder.make_tx().map_err(Error::CannotBuildTxFromBuilder)?;
        let finalized = TxFinalized::new(tx);

        Ok((finalized, changes_used))
    }

    /// update the transaction with the given operation
    pub fn update_with(&mut self, operation: Operation) -> Result<&mut Self> {
        match operation {
            Operation::AddInput(input)     => self.add_input(input),
            Operation::AddOutput(output)   => self.add_output(output),
            Operation::AddChange(change)   => self.add_change(change),
            Operation::RemoveInput(txin)   => self.remove_input(txin),
            Operation::RemoveOutput(index) => self.remove_output(index),
            Operation::RemoveChange(addr)  => self.remove_change(addr),
            Operation::Signature(witness)  => self.add_witness(witness),
            Operation::Finalize            => self.finalize(),
        }
    }

    /// accessor to all of the transaction's inputs.
    pub fn inputs<'a>(&'a self) -> &'a [Input] { self.inputs.as_ref() }

    /// accessor to all of the transaction's outputs. Ordered as it is in the
    /// transaction.
    pub fn outputs<'a>(&'a self) -> &'a [Output] { self.outputs.as_ref() }

    /// returns reference to the change
    pub fn changes<'a>(&'a self) -> &'a [Change] { self.changes.as_ref() }

    pub fn signature<'a>(&'a self) -> &'a [TxInWitness] { self.witnesses.as_ref() }

    pub fn has_change(&self) -> bool { ! self.changes.is_empty() }

    pub fn is_finalized(&self) -> bool { self.finalized }

    /// lookup the inputs for the given `TxoPointer`
    pub fn lookup_input(&self, txin: TxoPointer) -> Option<usize> {
        self.inputs().iter().position(|input| &input.extract_txin() == &txin)
    }

    fn add_output(&mut self, output: Output) -> Result<&mut Self> {
        if self.is_finalized() { return Err(Error::CannotAddOutputToAFinalizedTransaction); }
        self.outputs.push(output);
        Ok(self)
    }
    fn add_input(&mut self, input: Input) -> Result<&mut Self> {
        if self.is_finalized() { return Err(Error::CannotAddInputsToAFinalizedTransaction); }
        self.inputs.push(input);
        Ok(self)
    }
    fn add_change(&mut self, change: Change) -> Result<&mut Self> {
        if self.is_finalized() { return Err(Error::CannotAddChangeToAFinalizedTransaction); }
        if ! self.changes.is_empty() { return Err(Error::MoreThanOneChangeAddressIsNotSupportedYet); }
        self.changes.push(change);
        Ok(self)
    }
    fn add_witness(&mut self, witness: TxInWitness) -> Result<&mut Self> {
        if ! self.is_finalized() { return Err(Error::CannotAddWitnessesToAnOpenedTransaction); }
        if self.inputs.len() <= self.witnesses.len() {
            return Err(Error::CannotAddMoreWitnessesThanInputs);
        }
        self.witnesses.push(witness);
        Ok(self)
    }

    fn remove_input(&mut self, txin: TxoPointer) -> Result<&mut Self> {
        // Here we could have used Drain Filter, but the feature is still not stable.
        // [see rust lang's issue #43244](https://github.com/rust-lang/rust/issues/43244).
        //
        // In the meanwhile the following is just as good.

        let mut index = 0;

        let mut removed = false;

        // we are not using `0..inputs.len()` because we are potentially removing
        // items as we go along
        while index != self.inputs.len() {
            if self.inputs[index].extract_txin() == txin {
                let input = self.inputs.remove(index);
                removed = true;
                debug!("removing input: {:#?}", input);
            } else { index += 1; }
        }

        if ! removed {
            Err(Error::CannotRemoveInputInputNotFound)
        } else {
            Ok(self)
        }
    }

    fn remove_output(&mut self, index: u32) -> Result<&mut Self> {
        if self.outputs.len() < index as usize {
            return Err(Error::CannotRemoveOutput);
        }

        let output = self.outputs.remove(index as usize);

        debug!("removing outputs {:#?}", output);

        Ok(self)
    }

    fn remove_change(&mut self, addr: ExtendedAddr) -> Result<& mut Self> {
        let mut index = 0;

        let mut removed = false;

        // we are not using `0..inputs.len()` because we are potentially removing
        // items as we go along
        while index != self.changes.len() {
            if self.changes[index].address == addr {
                let change = self.changes.remove(index);
                removed = true;
                debug!("removing change: {:#?}", change);
            } else { index += 1; }
        }

        if ! removed {
            Err(Error::CannotRemoveChangeChangeNotFound)
        } else {
            Ok(self)
        }
    }

    pub fn finalize(&mut self) -> Result<& mut Self> {
        if self.finalized { return Err(Error::CannotFinalizeAFinalizedTransaction); }
        self.finalized = true;
        Ok(self)
    }


}
impl Default for Transaction {
    fn default() -> Self { Transaction::new() }
}
