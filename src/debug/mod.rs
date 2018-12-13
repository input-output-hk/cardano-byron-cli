use cardano::{address::{ExtendedAddr, StakeDistribution}, util::{base58, hex, try_from_slice::{TryFromSlice}}, hash};
use cardano::hdwallet;
use utils::term::{emoji, Term, style::Style};
use std::{io::{self, Read, Write}};
use exe_common::parse_genesis_data;
use rand;

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

pub fn decode_signed_tx() {
    let mut data = String::new();
    io::stdin().read_to_string(&mut data).expect("Cannot read stdin.");

    let bytes = base64::decode(&data).unwrap();
    let txaux : cardano::tx::TxAux = cbor_event::de::RawCbor::from(&bytes).deserialize_complete().unwrap();

    println!("inputs({})", txaux.tx.inputs.len());
    for ((i, input), witness) in txaux.tx.inputs.iter().enumerate().zip(txaux.witness.iter()) {
        let signature_ok = witness.verify_tx(Default::default(), &txaux.tx);
        let valid = if signature_ok {
            emoji::CHECK_MARK
        } else {
            emoji::CROSS_MARK
        };
        println!(" - input ({}) {}.{} {}", i, style!(&input.id), style!(&input.index), valid);
    }

    println!("outputs({}):", txaux.tx.outputs.len());
    for (i, output) in txaux.tx.outputs.iter().enumerate() {
        println!(" - output ({}) {} {}", i, style!(&output.address), style!(&output.value));
    }
}

pub fn generate_xprv(output_prv: &str) {
    let mut buf = [0u8;hdwallet::XPRV_SIZE];
    for x in buf.iter_mut() {
        *x = rand::random()
    }

    let xprv = hdwallet::XPrv::normalize_bytes(buf);
    let s = hex::encode(xprv.as_ref());

    let mut file = ::std::fs::OpenOptions::new().create(true).write(true).open(output_prv).unwrap();
    file.write_all(s.as_ref()).unwrap();
}

pub fn xprv_to_xpub(input_priv: &str, output_pub: &str) {
    let xprv = {
        let mut infile = ::std::fs::OpenOptions::new().read(true).open(input_priv).unwrap();
        let mut v = Vec::new();
        infile.read_to_end(&mut v).unwrap();
        let s = String::from_utf8(v).unwrap();
        let i = hex::decode(&s).unwrap();
        if i.len() != hdwallet::XPRV_SIZE {
            panic!("not containing an XPRV")
        }

        let mut xprv_data = [0u8;hdwallet::XPRV_SIZE];
        xprv_data.copy_from_slice(&i[..]);
        hdwallet::XPrv::from_bytes_verified(xprv_data).unwrap()
    };
    let xpub = xprv.public();
    let s = hex::encode(xpub.as_ref());
    let mut file = ::std::fs::OpenOptions::new().create(true).write(true).open(output_pub).unwrap();
    file.write_all(s.as_ref()).unwrap();
    ()
}
