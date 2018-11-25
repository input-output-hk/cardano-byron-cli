use super::config::{encrypt_primary_key, Config, HDWalletModel};
use super::{WalletName, Wallet, Wallets};
use super::error::{Error, Result};
use super::state::{lookup};
use super::utils::{*};

use std::{path::PathBuf, io::Write};
use cardano::{hdwallet::{self, DerivationScheme}, wallet, bip::bip39};
use rand::random;

use utils::{term::{Term, style::{Style}}, prompt};

use blockchain::{Blockchain, BlockchainName};

pub fn list(
    term: &mut Term,
    root_dir: PathBuf,
    detailed: bool,
) -> Result<()> {
    let wallets = Wallets::load(root_dir.clone())?;
    for (_, wallet) in wallets {
        let detail = if detailed {
            if let Some(blk_name) = &wallet.config.attached_blockchain {
                let state = create_wallet_state_from_logs(
                    &wallet,
                    &root_dir,
                    lookup::accum::Accum::default(),
                )?;

                let total = state.total()?;

                format!("\t{}\t{}@{}",
                    style!(total).green().bold(),
                    style!(blk_name).underlined().white(),
                    style!(state.ptr.latest_block_date())
                )
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        writeln!(term, "{}{}", style!(wallet.name).cyan().italic(), detail).unwrap();
    }
    Ok(())
}

/// function to create a new wallet
///
pub fn new<D>(
    term: &mut Term,
    root_dir: PathBuf,
    name: WalletName,
    wallet_scheme: HDWalletModel,
    derivation_scheme: DerivationScheme,
    mnemonic_size: bip39::Type,
    languages: Vec<D>,
) -> Result<()>
where
    D: bip39::dictionary::Language,
{
    let config = Config {
        attached_blockchain: None,
        derivation_scheme: derivation_scheme,
        hdwallet_model: wallet_scheme
    };

    // 1. generate the mnemonics

    let entropy = bip39::Entropy::generate(mnemonic_size, random);
    // 2. perform the seed generation from the entropy

    term.info("You can add a recovery wallet password. You can set no password, however you won't benefit from plausible deniability\n").unwrap();
    let recovery_password = term.new_password("recovery password", "confirm password", "password mismatch ").unwrap();
    let mut seed = [0;hdwallet::XPRV_SIZE];
    wallet::keygen::generate_seed(&entropy, recovery_password.as_bytes(), &mut seed);

    term.info("Please, note carefully the following mnemonic words. They will be needed to recover your wallet.\n").unwrap();
    for lang in languages {
        term.warn(&format!("{}: ", lang.name())).unwrap();
        let mnemonic_phrase = entropy.to_mnemonics().to_string(&lang);
        term.simply(&format!("{}\n", mnemonic_phrase)).unwrap();
    }

    // 3. normalize the seed to make it a valid private key

    let xprv = hdwallet::XPrv::normalize_bytes(seed);

    // create the root public key
    let public_key = match wallet_scheme {
        HDWalletModel::BIP44 => None,
        HDWalletModel::RandomIndex2Levels => Some(xprv.public()),
    };

    // 4. encrypt the private key
    term.info("Set a wallet password. This is for local usage only, allows you to protect your cached private key and prevent from creating non desired transactions.\n").unwrap();
    let password = term.new_password("spending password", "confirm spending password", "password mismatch").unwrap();
    let encrypted_xprv = encrypt_primary_key(password.as_bytes(), &xprv);

    // 5. create the wallet
    let wallet = Wallet::new(root_dir, name, config, encrypted_xprv, public_key);

    // 6. save the wallet
    wallet.save()?;

    term.success(&format!("wallet `{}' successfully created.\n", &wallet.name)).unwrap();

    Ok(())
}

pub fn recover<D>(
    term: &mut Term,
    root_dir: PathBuf,
    name: WalletName,
    wallet_scheme: HDWalletModel,
    derivation_scheme: DerivationScheme,
    mnemonic_size: bip39::Type,
    interactive: bool,
    daedalus_seed: bool,
    language: D,
) -> Result<()>
where
    D: bip39::dictionary::Language,
{
    let config = Config {
        attached_blockchain: None,
        derivation_scheme: derivation_scheme,
        hdwallet_model: wallet_scheme
    };

    // 1. generate the mnemonics
    term.info("enter your mnemonics\n").unwrap();

    let (string, _, entropy) = if interactive {
        prompt::mnemonics::interactive_input_words(term, &language, mnemonic_size)
    } else {
        prompt::mnemonics::input_mnemonic_phrase(term, &language, mnemonic_size)
    };

    // 3. perform the seed generation from the entropy
    let xprv = if daedalus_seed {
        match wallet::rindex::RootKey::from_daedalus_mnemonics(
            derivation_scheme,
            &language,
            string.to_string(),
        ) {
            Ok(root_key) => (*root_key).clone(),
            Err(e) => {
                return Err(Error::CannotRecoverFromDaedalusMnemonics(e));
            }
        }
    } else {
        term.info("Enter the wallet recovery password (if the password is wrong, you won't know).\n").unwrap();
        let recovery_password = term.password("recovery password: ").unwrap();

        let mut seed = [0;hdwallet::XPRV_SIZE];
        wallet::keygen::generate_seed(&entropy, recovery_password.as_bytes(), &mut seed);

        // normalize the seed to make it a valid private key
        hdwallet::XPrv::normalize_bytes(seed)
    };

    // create the root public key
    let public_key = match wallet_scheme {
        HDWalletModel::BIP44 => None,
        HDWalletModel::RandomIndex2Levels => Some(xprv.public()),
    };

    // 4. encrypt the private key
    term.info("Set a wallet password. This is for local usage only, allows you to protect your cached private key and prevent from creating non desired transactions.\n").unwrap();
    let password = term.new_password("spending password", "confirm spending password", "password mismatch").unwrap();
    let encrypted_xprv = encrypt_primary_key(password.as_bytes(), &xprv);

    // 5. create the wallet
    let wallet = Wallet::new(root_dir, name, config, encrypted_xprv, public_key);

    // 6. save the wallet
    wallet.save()?;

    term.success(&format!("wallet `{}' successfully recovered.\n", &wallet.name)).unwrap();

    Ok(())
}

/// Destroy the wallet and remove all associated data.
///
/// **Caveat:** the files in storage are only unlinked on the filesystem
/// level, the data on the physical medium are not erased.
///
pub fn destroy(
    term: &mut Term,
    root_dir: PathBuf,
    name: WalletName,
) -> Result<()> {
    // load the wallet
    let wallet = Wallet::load(&root_dir, name)?;

    writeln!(term, "You are about to destroy your wallet {}.
This means that all the data associated to this wallet will be deleted on this device.
The only way you will be able to reuse the wallet, recover the funds and create
new transactions will be by recovering the wallet with the mnemonic words.",
        ::console::style(&wallet.name).bold().red(),
    ).unwrap();

    // FIXME: create a confirmation API that is exposed on the command
    // methods that need to put the user through a confirmation dialog.
    // See issue #45

    let confirmation = ::dialoguer::Confirmation::new().with_text("Are you sure?")
        .default(false)
        .interact().unwrap();
    if ! confirmation { ::std::process::exit(0); }

    wallet.destroy().map_err(|e| Error::WalletDestroyFailed(e))?;

    term.success("Wallet successfully destroyed.\n").unwrap();

    Ok(())
}

pub fn attach(
    term: &mut Term,
    root_dir: PathBuf,
    name: WalletName,
    blockchain_name: BlockchainName
) -> Result<()> {
    // load the wallet
    let mut wallet = Wallet::load(&root_dir, name)?;

    // 1. is the wallet already attached
    if let Some(ref bn) = wallet.config.attached_blockchain {
        return Err(Error::AttachAlreadyAttached(bn.clone()));
    }

    // 2. check the blockchain exists
    Blockchain::load(&root_dir, blockchain_name.clone())?;

    // 3. save the attached wallet
    wallet.config.attached_blockchain = Some(blockchain_name.as_ref().to_owned());
    wallet.save()?;

    term.success("Wallet successfully attached to blockchain.\n").unwrap();

    Ok(())
}

pub fn detach(
    term: &mut Term,
    root_dir: PathBuf,
    name: WalletName,
) -> Result<()> {
    // load the wallet
    let mut wallet = Wallet::load(&root_dir, name)?;

    // 1. get the wallet's blockchain
    let _ = load_attached_blockchain(
        &root_dir,
        &wallet.config,
    )?;

    // 2. delete the wallet log
    wallet.delete_log()?;

    wallet.config.attached_blockchain = None;

    wallet.save()?;

    term.success("Wallet successfully detached from blockchain.\n").unwrap();

    Ok(())
}

pub fn status(
    term: &mut Term,
    root_dir: PathBuf,
    name: WalletName,
) -> Result<()> {
    // load the wallet
    let wallet = Wallet::load(root_dir.clone(), name)?;

    if let Some(ref blk_name) = &wallet.config.attached_blockchain {
        term.simply("Wallet ").unwrap();
        term.warn(&format!("{}", &wallet.name)).unwrap();
        term.simply(" on blockchain ").unwrap();
        term.info(blk_name).unwrap();
        term.simply("\n").unwrap();
    } else {
        term.info(&format!("Wallet {} status\n", &wallet.name)).unwrap();
        term.warn("wallet not attached to a blockchain\n").unwrap();
        return Ok(());
    }

    term.simply(" * wallet model ").unwrap();
    term.warn(&format!("{:?}", &wallet.config.hdwallet_model)).unwrap();
    term.simply("\n").unwrap();
    term.simply(" * derivation scheme ").unwrap();
    term.warn(&format!("{:?}", &wallet.config.derivation_scheme)).unwrap();
    term.simply("\n").unwrap();

    let state = create_wallet_state_from_logs(
        &wallet,
        root_dir,
        lookup::accum::Accum::default(),
    )?;

    let total = state.total()?;

    term.simply(" * balance ").unwrap();
    term.success(&format!(" {}", total)).unwrap();
    term.simply("\n").unwrap();
    match state.ptr.latest_addr {
        Some(latest_addr) => {
            term.simply(" * synced to block ").unwrap();
            term.warn(&format!(
                " {} ({})",
                state.ptr.latest_known_hash,
                latest_addr,
            )).unwrap();
        }
        None => {
            term.simply(" * ").unwrap();
            term.warn("not synced yet").unwrap();
        }
    }
    term.simply("\n").unwrap();

    Ok(())
}

pub fn log(term: &mut Term,
    root_dir: PathBuf,
    name: WalletName,
    pretty: bool,
) -> Result<()> {
    // load the wallet
    let wallet = Wallet::load(root_dir.clone(), name)?;

    let mut state = create_wallet_state_from_logs(
        &wallet,
        &root_dir,
        lookup::accum::Accum::default(),
    )?;

    display_wallet_state_logs(term, &wallet, &mut state, pretty);

    Ok(())
}

pub fn utxos(term: &mut Term,
    root_dir: PathBuf,
    name: WalletName,
) -> Result<()> {
    // load the wallet
    let wallet = Wallet::load(root_dir.clone(), name)?;

    let state = create_wallet_state_from_logs(
        &wallet,
        &root_dir,
        lookup::accum::Accum::default(),
    )?;

    display_wallet_state_utxos(term, state);

    Ok(())
}

pub fn sync(term: &mut Term,
    root_dir: PathBuf,
    name: WalletName,
) -> Result<()> {
    // 0. load the wallet
    let wallet = Wallet::load(root_dir.clone(), name)?;

    // 1. get the wallet's blockchain
    let blockchain = load_attached_blockchain(&root_dir, &wallet.config)?;

    match wallet.config.hdwallet_model {
        HDWalletModel::BIP44 => {
            let mut lookup_struct = load_bip44_lookup_structure(term, &wallet);
            lookup_struct.prepare_next_account()?;

            let mut state = create_wallet_state_from_logs(
                &wallet,
                &root_dir,
                lookup_struct,
            )?;

            update_wallet_state_with_utxos(term, &wallet, &blockchain, &mut state);
        },
        HDWalletModel::RandomIndex2Levels => {
            let lookup_struct = load_randomindex_lookup_structure(term, &wallet);
            let mut state = create_wallet_state_from_logs(
                &wallet,
                &root_dir,
                lookup_struct,
            )?;

            update_wallet_state_with_utxos(term, &wallet, &blockchain, &mut state);
        },
    };

    Ok(())
}

pub fn address(
    term: &mut Term,
    root_dir: PathBuf,
    name: WalletName,
    account: u32,
    is_internal: bool,
    index: u32,
) -> Result<()> {
    // load the wallet
    let wallet = Wallet::load(root_dir.clone(), name)?;

    let addr = match wallet.config.hdwallet_model {
        HDWalletModel::BIP44 => {
            let mut lookup_struct = load_bip44_lookup_structure(term, &wallet);
            let account = ::cardano::bip::bip44::Account::new(account)?;
            let change = if is_internal {
                account.internal()?
            } else {
                account.external()?
            };
            let addressing = change.index(index)?;
            lookup_struct.get_address(&addressing)
        },
        HDWalletModel::RandomIndex2Levels => {
            let lookup_struct = load_randomindex_lookup_structure(term, &wallet);
            let addressing = ::cardano::wallet::rindex::Addressing::new(account, index);
            lookup_struct.get_address(&addressing)
        }
    };

    writeln!(term, "{}", style!(addr));

    Ok(())
}
