use cardano::{address::{ExtendedAddr, StakeDistribution}, util::{base58, hex, try_from_slice::{TryFromSlice}}, hash};
use cardano_storage::utxo;
use utils::term::Term;
use std::io::{self, Read};
use exe_common::parse_genesis_data;

pub fn command_address( mut term: Term
                      , address: String
                      )
{
    let bytes = match base58::decode(&address) {
        Err(err) => {
            term.error(&format!("Invalid Address, should be encoded in base58\n")).unwrap();
            term.error(&format!("{}\n", err)).unwrap();
            ::std::process::exit(1)
        },
        Ok(bytes) => bytes,
    };

    let address = match ExtendedAddr::try_from_slice(&bytes) {
        Err(err) => {
            term.error(&format!("Invalid Address\n")).unwrap();
            term.error(&format!("{:?}\n", err)).unwrap();
            ::std::process::exit(2)
        },
        Ok(address) => address,
    };

    term.success("Cardano Extended Address\n").unwrap();
    term.info(&format!("  - address hash:       {}\n", address.addr)).unwrap();
    term.info(&format!("  - address type:       {}\n", address.addr_type)).unwrap();
    if let Some(ref payload) = address.attributes.derivation_path {
        term.info(&format!("  - payload:            {}\n", hex::encode(payload.as_ref()))).unwrap();
    }
    match address.attributes.stake_distribution {
        StakeDistribution::BootstrapEraDistr =>
           term.info("  - stake distribution: bootstrap era\n").unwrap(),
        StakeDistribution::SingleKeyDistr(id) =>
           term.info(&format!("  - stake distribution: {}\n", id)).unwrap(),
    }
}

/// Read a JSON file from stdin and write its canonicalized form to stdout.
pub fn canonicalize_json()
{
    let mut json = String::new();
    io::stdin().read_to_string(&mut json).expect("Cannot read stdin.");
    print!("{}", parse_genesis_data::canonicalize_json(json.as_bytes()));
}

/// Compute the Blake2b256 hash of the data on stdin.
pub fn hash()
{
    let mut data = vec![];
    io::stdin().read_to_end(&mut data).expect("Cannot read stdin.");
    println!("{}", hash::Blake2b256::new(&data));
}

pub fn decode_utxos() {
    let mut data = vec![];
    io::stdin().read_to_end(&mut data).expect("Cannot read stdin.");
    println!("{:?}", utxo::decode_utxo_file(&mut &data[..]).unwrap());
}
