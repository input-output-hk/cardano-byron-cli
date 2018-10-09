use std::{io::{Write, Result}};

use cardano::block::{genesis, normal, types, Block};
use cardano::{address, tx};

use super::term::style::{Style, StyledObject};

// Constants for the fmt::Display instance
pub const DISPLAY_INDENT_SIZE: usize = 4; // spaces

pub trait Pretty {
    fn pretty<W>(self, f: &mut W, indent: usize) -> Result<()>
        where W: Write;
}

fn pretty_attribute<P: Pretty, W: Write>(w: &mut W, indent: usize, k: &'static str, v: P) -> Result<()> {
    write!(w, "{:width$}\"{}\": ", "", k, width = indent)?;
    v.pretty(w, indent + DISPLAY_INDENT_SIZE)?;
    Ok(())
}

fn pretty_obj_start<W: Write>(w: &mut W, indent: usize) -> Result<()> {
    let adjusted_indent = if indent >= DISPLAY_INDENT_SIZE {
        indent - DISPLAY_INDENT_SIZE
    } else {
        indent
    };
    writeln!(w, "\n{:width$}{}{{", "", "", width = adjusted_indent)?;
    Ok(())
}
fn pretty_obj_end<W: Write>(w: &mut W, indent: usize) -> Result<()> {
    let adjusted_indent = if indent >= DISPLAY_INDENT_SIZE {
        indent - DISPLAY_INDENT_SIZE
    } else {
        indent
    };
    write!(w, "\n{:width$}{}}}", "", "", width = adjusted_indent)?;
    Ok(())
}

impl<'a> Pretty for &'a str {
    fn pretty<W>(self, f: &mut W, _: usize) -> Result<()>
        where W: Write
    {
        write!(f, "\"{}\"", self)
    }
}

impl<D: ::std::fmt::Display> Pretty for StyledObject<D> {
    fn pretty<W>(self, f: &mut W, _: usize) -> Result<()>
        where W: Write
    {
        write!(f, "\"{}\"", self)
    }
}

impl<'a, D: ::std::fmt::Display> Pretty for &'a StyledObject<D> {
    fn pretty<W>(self, f: &mut W, _: usize) -> Result<()>
        where W: Write
    {
        write!(f, "\"{}\"", self)
    }
}

fn pretty_iterator<I, D, W>(w: &mut W, indent: usize, iter: I) -> Result<()>
    where I: IntoIterator<Item = D>
        , D: Pretty
        , W: Write
{
    write!(w, "\n{:width$}[", "", width = indent)?;

    // get first item to solve fenceposting the comma
    let mut iterator = iter.into_iter();
    match iterator.next() {
        Some(item) => {
            item.pretty(w, indent + 2*DISPLAY_INDENT_SIZE)?;
        }
        _ => {}
    }
    loop {
        match iterator.next()
        {
            Some(item) => {
                write!(w, ",")?;
                write!(w, "{:width$}", "", width = indent)?;
                item.pretty(w, indent + 2*DISPLAY_INDENT_SIZE)?;
            }
            None => break
        }
    }
    write!(w, "\n{:width$}]", "", width = indent)?;
    Ok(())
}

impl<D: Pretty> Pretty for Vec<D> {
    fn pretty<W>(self, f: &mut W, indent: usize) -> Result<()>
        where W: Write
    {
        pretty_iterator(f, indent, self.into_iter())
    }
}

impl Pretty for Block {
    fn pretty<W>(self, f: &mut W, indent: usize) -> Result<()>
        where W: Write
    {
        pretty_obj_start(f, indent)?;
        match self {
            Block::GenesisBlock(blk) => 
            {
                pretty_attribute(f, indent, "gen_block", blk)?;
            }
            Block::MainBlock(blk) => {
                pretty_attribute(f, indent, "block", blk)?;
            }
        }
        pretty_obj_end(f, indent)?;
        Ok(())
    }
}
impl Pretty for genesis::Block {
    fn pretty<W>(self, f: &mut W, indent: usize) -> Result<()>
        where W: Write
    {
        pretty_obj_start(f, indent)?;
        pretty_attribute(f, indent, "header", self.header)?;
        writeln!(f, ",")?;
        pretty_attribute(f, indent, "body", self.body)?;
        // TODO: extra?
        pretty_obj_end(f, indent)?;
        Ok(())
    }
}
impl Pretty for normal::Block {
    fn pretty<W>(self, f: &mut W, indent: usize) -> Result<()>
        where W: Write
    {
        pretty_obj_start(f, indent)?;
        pretty_attribute(f, indent, "header", self.header)?;
        writeln!(f, ",")?;
        pretty_attribute(f, indent, "body", self.body)?;
        // TODO: extra?
        pretty_obj_end(f, indent)?;
        Ok(())
    }
}
impl Pretty for genesis::Body {
    fn pretty<W>(self, f: &mut W, indent: usize) -> Result<()>
        where W: Write
    {
        pretty_obj_start(f, indent)?;
        // pretty_attribute(f, indent, "ssc", self.ssc)?;
        pretty_attribute(f, indent, "slot_leaders", self.slot_leaders)?;
        // TODO: delegation?
        // TODO: update?
        pretty_obj_end(f, indent)?;
        Ok(())
    }
}
impl Pretty for normal::Body {
    fn pretty<W>(self, f: &mut W, indent: usize) -> Result<()>
        where W: Write
    {
        pretty_obj_start(f, indent)?;
        // pretty_attribute(f, indent, "ssc", self.ssc)?;
        pretty_attribute(f, indent, "txs", self.tx)?;
        // TODO: delegation?
        // TODO: update?
        pretty_obj_end(f, indent)?;
        Ok(())
    }
}
impl Pretty for normal::TxPayload {
    fn pretty<W>(self, f: &mut W, indent: usize) -> Result<()>
        where W: Write
    {
        pretty_iterator(f, indent, self.into_iter())
    }
}
impl Pretty for tx::TxAux {
    fn pretty<W>(self, f: &mut W, indent: usize) -> Result<()>
        where W: Write
    {
        pretty_obj_start(f, indent)?;
        pretty_attribute(f, indent, "tx", self.tx)?;
        writeln!(f, ",")?;
        pretty_attribute(f, indent, "witnesses", self.witness.to_vec())?;
        pretty_obj_end(f, indent)?;
        Ok(())
    }
}
impl Pretty for tx::Tx {
    fn pretty<W>(self, f: &mut W, indent: usize) -> Result<()>
        where W: Write
    {
        pretty_obj_start(f, indent)?;
        pretty_attribute(f, indent, "inputs", self.inputs)?;
        writeln!(f, ",")?;
        pretty_attribute(f, indent, "outputs", self.outputs)?;
        pretty_obj_end(f, indent)?;
        Ok(())
    }
}
impl Pretty for tx::TxoPointer {
    fn pretty<W>(self, f: &mut W, indent: usize) -> Result<()>
        where W: Write
    {
        pretty_obj_start(f, indent)?;
        pretty_attribute(f, indent, "id", style!(self.id))?;
        writeln!(f, ",")?;
        pretty_attribute(f, indent, "index", style!(self.index))?;
        pretty_obj_end(f, indent)?;
        Ok(())
    }
}
impl Pretty for tx::TxOut {
    fn pretty<W>(self, f: &mut W, indent: usize) -> Result<()>
        where W: Write
    {
        pretty_obj_start(f, indent)?;
        pretty_attribute(f, indent, "address", style!(self.address))?;
        writeln!(f, ",")?;
        pretty_attribute(f, indent, "value", style!(self.value))?;
        pretty_obj_end(f, indent)?;
        Ok(())
    }
}
impl Pretty for tx::TxInWitness {
    fn pretty<W>(self, f: &mut W, indent: usize) -> Result<()>
        where W: Write
    {
        pretty_obj_start(f, indent)?;
        match self {
            tx::TxInWitness::PkWitness(xpub, signature) => {
                pretty_attribute(f, indent, "xpub", style!(xpub))?;
                writeln!(f, ",")?;
                pretty_attribute(f, indent, "signature", style!(signature))?;
                writeln!(f, ",")?;
                pretty_attribute(f, indent, "type", style!("Public Key"))?;
            },
            tx::TxInWitness::ScriptWitness(_, _) => {
                pretty_attribute(f, indent, "type", style!("Script"))?;
            },
            tx::TxInWitness::RedeemWitness(public, signature) => {
                pretty_attribute(f, indent, "type", style!("Redeem"))?;
            },
        }
        pretty_obj_end(f, indent)?;
        Ok(())
    }
}
impl Pretty for address::StakeholderId {
    fn pretty<W>(self, f: &mut W, _: usize) -> Result<()>
        where W: Write
    {
        write!(f, "\"{}\"", style!(self))
    }
}

impl Pretty for genesis::BlockHeader {
    fn pretty<W>(self, f: &mut W, indent: usize) -> Result<()>
        where W: Write
    {
        pretty_obj_start(f, indent)?;
        pretty_attribute(f, indent, "protocol_magic", style!(self.protocol_magic))?;
        writeln!(f, ",")?;
        pretty_attribute(f, indent, "previous_header", style!(self.previous_header))?;
        writeln!(f, ",")?;
        pretty_attribute(f, indent, "body_proof", style!(self.body_proof))?;
        writeln!(f, ",")?;
        pretty_attribute(f, indent, "consensus", self.consensus)?;
        // pretty_attribute(f, indent, "extra_data", self.extra_data)?;
        pretty_obj_end(f, indent)?;
        Ok(())
    }
}
impl Pretty for normal::BlockHeader {
    fn pretty<W>(self, f: &mut W, indent: usize) -> Result<()>
        where W: Write
    {
        pretty_obj_start(f, indent)?;
        pretty_attribute(f, indent, "protocol_magic", style!(self.protocol_magic))?;
        writeln!(f, ",")?;
        pretty_attribute(f, indent, "previous_header", style!(self.previous_header))?;
        writeln!(f, ",")?;
        pretty_attribute(f, indent, "body_proof", self.body_proof)?;
        writeln!(f, ",")?;
        pretty_attribute(f, indent, "consensus", self.consensus)?;
        pretty_obj_end(f, indent)?;
        Ok(())
    }
}

impl Pretty for genesis::Consensus {
    fn pretty<W>(self, f: &mut W, indent: usize) -> Result<()>
        where W: Write
    {
        pretty_obj_start(f, indent)?;
        pretty_attribute(f, indent, "epochid", style!(self.epoch).red().bold())?;
        writeln!(f, ",")?;
        pretty_attribute(f, indent, "chain_difficulty", style!(self.chain_difficulty))?;
        pretty_obj_end(f, indent)?;
        Ok(())
    }
}
impl Pretty for normal::Consensus {
    fn pretty<W>(self, f: &mut W, indent: usize) -> Result<()>
        where W: Write
    {
        pretty_obj_start(f, indent)?;
        pretty_attribute(f, indent, "slotid", style!(self.slot_id))?;
        writeln!(f, ",")?;
        pretty_attribute(f, indent, "leader_key", style!(self.leader_key))?;
        writeln!(f, ",")?;
        pretty_attribute(f, indent, "chain_difficulty", style!(self.chain_difficulty))?;
        match self.block_signature {
            normal::BlockSignature::Signature(blk) => {
                writeln!(f, ",")?;
                pretty_attribute(f, indent, "block_signature", style!(blk))?;
            },
            _ => {
                // TODO
            }
        }
        pretty_obj_end(f, indent)?;
        Ok(())
    }
}

impl Pretty for normal::BodyProof {
    fn pretty<W>(self, f: &mut W, indent: usize) -> Result<()>
        where W: Write
    {
        pretty_obj_start(f, indent)?;
        pretty_attribute(f, indent, "tx", self.tx)?;
        writeln!(f, ",")?;
        pretty_attribute(f, indent, "mpc", self.mpc)?;
        writeln!(f, ",")?;
        pretty_attribute(f, indent, "proxy_sk", style!(self.proxy_sk))?;
        writeln!(f, ",")?;
        pretty_attribute(f, indent, "update", style!(self.update))?;
        pretty_obj_end(f, indent)?;
        Ok(())
    }
}
impl Pretty for tx::TxProof {
    fn pretty<W>(self, f: &mut W, indent: usize) -> Result<()>
        where W: Write
    {
        pretty_obj_start(f, indent)?;
        pretty_attribute(f, indent, "number", style!(self.number))?;
        writeln!(f, ",")?;
        pretty_attribute(f, indent, "root", style!(self.root))?;
        writeln!(f, ",")?;
        pretty_attribute(f, indent, "witnesses_hash", style!(self.witnesses_hash))?;
        pretty_obj_end(f, indent)?;
        Ok(())
    }
}
impl Pretty for types::SscProof {
    fn pretty<W>(self, f: &mut W, indent: usize) -> Result<()>
        where W: Write
    {
        pretty_obj_start(f, indent)?;
        match self {
            types::SscProof::Commitments(h1, h2) => {
                pretty_attribute(f, indent, "h1", style!(h1))?;
                writeln!(f, ",")?;
                pretty_attribute(f, indent, "h2", style!(h2))?;
                writeln!(f, ",")?;
                pretty_attribute(f, indent, "type", style!("Commitments"))?;
            },
            types::SscProof::Openings(h1, h2) => {
                pretty_attribute(f, indent, "h1", style!(h1))?;
                writeln!(f, ",")?;
                pretty_attribute(f, indent, "h2", style!(h2))?;
                writeln!(f, ",")?;
                pretty_attribute(f, indent, "type", style!("Openings"))?;
            },
            types::SscProof::Shares(h1, h2) => {
                pretty_attribute(f, indent, "h1", style!(h1))?;
                writeln!(f, ",")?;
                pretty_attribute(f, indent, "h2", style!(h2))?;
                writeln!(f, ",")?;
                pretty_attribute(f, indent, "type", style!("Shares"))?;
            },
            types::SscProof::Certificate(h1) => {
                pretty_attribute(f, indent, "h1", style!(h1))?;
                writeln!(f, ",")?;
                pretty_attribute(f, indent, "type", style!("Shares"))?;
            }
        }
        pretty_obj_end(f, indent)?;
        Ok(())
    }
}
