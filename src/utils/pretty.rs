use std::{io::{Write, Result}};

use cardano::block::{genesis, normal, types, Block};
use cardano::{address, tx};

use super::term::style::{Style, StyledObject};

// Constants for the fmt::Display instance
static DISPLAY_INDENT_SIZE: usize = 4; // spaces

pub trait Pretty {
    fn pretty<W>(self, f: &mut W, indent: usize) -> Result<()>
        where W: Write;
}

fn pretty_attribute<P: Pretty, W: Write>(w: &mut W, indent: usize, k: &'static str, v: P) -> Result<()> {
    write!(w, "{:width$}\"{}\": ", "", k, width = indent)?;
    v.pretty(w, indent + DISPLAY_INDENT_SIZE)?;
    writeln!(w, "")?;
    Ok(())
}

fn pretty_obj_start<W: Write>(w: &mut W, indent: usize) -> Result<()> {
    writeln!(w, "\n{:width$}{}{{", "", "", width = indent)?;
    Ok(())
}
fn pretty_obj_end<W: Write>(w: &mut W, indent: usize) -> Result<()> {
    writeln!(w, "{:width$}{}}},", "", "", width = indent)?;
    Ok(())
}

impl<'a> Pretty for &'a str {
    fn pretty<W>(self, f: &mut W, _: usize) -> Result<()>
        where W: Write
    {
        write!(f, "\"{}\",", self)
    }
}

impl<D: ::std::fmt::Display> Pretty for StyledObject<D> {
    fn pretty<W>(self, f: &mut W, _: usize) -> Result<()>
        where W: Write
    {
        write!(f, "\"{}\",", self)
    }
}

impl<'a, D: ::std::fmt::Display> Pretty for &'a StyledObject<D> {
    fn pretty<W>(self, f: &mut W, _: usize) -> Result<()>
        where W: Write
    {
        write!(f, "\"{}\",", self)
    }
}

fn pretty_iterator<I, D, W>(w: &mut W, indent: usize, iter: I) -> Result<()>
    where I: IntoIterator<Item = D>
        , D: Pretty
        , W: Write
{
    write!(w, "\n{:width$}[", "", width = indent)?;
    for e in iter {
        write!(w, "{:width$}", "", width = indent)?;
        e.pretty(w, indent + DISPLAY_INDENT_SIZE)?;
    }
    write!(w, "{:width$}],", "", width = indent)?;
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
                pretty_attribute(f, indent, "signature", style!(signature))?;
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
        write!(f, "\"{}\",", style!(self))
    }
}

impl Pretty for genesis::BlockHeader {
    fn pretty<W>(self, f: &mut W, indent: usize) -> Result<()>
        where W: Write
    {
        pretty_obj_start(f, indent)?;
        pretty_attribute(f, indent, "protocol_magic", style!(self.protocol_magic))?;
        pretty_attribute(f, indent, "previous_header", style!(self.previous_header))?;
        pretty_attribute(f, indent, "body_proof", style!(self.body_proof))?;
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
        pretty_attribute(f, indent, "previous_header", style!(self.previous_header))?;
        pretty_attribute(f, indent, "body_proof", self.body_proof)?;
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
        pretty_attribute(f, indent, "leader_key", style!(self.leader_key))?;
        pretty_attribute(f, indent, "chain_difficulty", style!(self.chain_difficulty))?;
        match self.block_signature {
            normal::BlockSignature::Signature(blk) => {
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
        pretty_attribute(f, indent, "mpc", self.mpc)?;
        pretty_attribute(f, indent, "proxy_sk", style!(self.proxy_sk))?;
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
        pretty_attribute(f, indent, "root", style!(self.root))?;
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
                pretty_attribute(f, indent, "h2", style!(h2))?;
                pretty_attribute(f, indent, "type", style!("Commitments"))?;
            },
            types::SscProof::Openings(h1, h2) => {
                pretty_attribute(f, indent, "h1", style!(h1))?;
                pretty_attribute(f, indent, "h2", style!(h2))?;
                pretty_attribute(f, indent, "type", style!("Openings"))?;
            },
            types::SscProof::Shares(h1, h2) => {
                pretty_attribute(f, indent, "h1", style!(h1))?;
                pretty_attribute(f, indent, "h2", style!(h2))?;
                pretty_attribute(f, indent, "type", style!("Shares"))?;
            },
            types::SscProof::Certificate(h1) => {
                pretty_attribute(f, indent, "h1", style!(h1))?;
                pretty_attribute(f, indent, "type", style!("Shares"))?;
            }
        }
        pretty_obj_end(f, indent)?;
        Ok(())
    }
}
