The command line is split into multiple subcommands

* `blockchain`: all the blockchain related tooling;
* `wallet`: create, recover and manage wallets;
* `transaction`: to build, review and sign transactions;
* `debug`: extra handy tooling.

# global options and environment variables

Within the global options, the more interesting ones are the

* `--root-dir=<PATH>` or the environment variable `CARDANO_CLI_ROOT_DIR`.
  It sets the directory where all storage, configuration and keys
  are. The default is to use appropriate directory depending of
  the operating system you are using:
  - the [XDG base directory](https://standards.freedesktop.org/basedir-spec/basedir-spec-latest.html) and
  the [XDG user directory](https://www.freedesktop.org/wiki/Software/xdg-user-dirs/) specifications on Linux
  - the [Known Folder](https://msdn.microsoft.com/en-us/library/windows/desktop/dd378457.aspx) API on Windows
  - the [Standard Directories](https://developer.apple.com/library/content/documentation/FileManagement/Conceptual/FileSystemProgrammingGuide/FileSystemOverview/FileSystemOverview.html#//apple_ref/doc/uid/TP40010672-CH2-SW6)
  guidelines on macOS
* `--color=<VALUE>` is the command to force using or not colored output
  in the terminal. The default is to automatically detect if it is a
  user managed terminal or not.

## FLAGS:

* `--quiet`      run the command quietly, do not print anything to the command line output
* `-v`, `--verbose`    set the verbosity mode, multiple occurrences means more verbosity
* `-h`, `--help`       Prints help information
* `-V`, `--version`    Prints version information

## OPTIONS:

* `--color <COLOR>`          enable output colors or not [default: auto]  [possible values: auto, always, never]
* `--root-dir <ROOT_DIR>`    the project root direction [env: CARDANO_CLI_ROOT_DIR=]  [default: ${HOME}/.local/share/cardano-cli]

# Guide

## `blockchain` command

### Local blockchains

This is the foundation on which the **CLI** is based on. You need to have a local
copy of the blockchain you are working with. You can have many local copies of any
of the cardano blockchains (`mainnet`, `testnet` or `staging`).

The storage of the blockchain has been done in an efficient way. Utilising less disk
space than other of the cardano tooling yet.

To list all the local blockchains, simply use the following command:

```
cardano-cli blockchain list
```

_hint_: add `--detailed` for more general information regarding the local blockchains

You can create as many local blockchains as you want (limited to space disk though) and because
it is allowed to have 1 or more copies of mainnet you can use aliases to these blockchains.

```
cardano-cli blockchain new mainnet-copy-1
```

The blockchain won't sync just yet. Have a look at `remote-fetch` or `pull` commands.

To create a testnet or a staging, simply use the `--template=testnet` or `--template=staging`
option when using the `blockchain new` command.

### Syncing blocks and managing remotes

When you have created your first copy of a blockchain, you will find that there is no blocks
in the blockchain just yet. This is because you need to fetch the blocks from other peers.

By default, we add the IOHK peers when utilising the `blockchain new` command. But if you wish
to add more, you can add them with the following command:

```
cardano-cli blockchain remote-add my-remote-alias my.remote.cardano.blockchain.local:9912
```

To list all the available remotes of a given blockchain, simply use `remote-ls`:

```
cardano-cli blockchain remote-ls testnet`
```

_hint_: try `--detailed` for more details about the status of the remotes, and `--complete`
(necessitate network connection) for a comparative review of the local states and the remotes'.

And now, to fetch the block you can either use `remote-sync` or `pull` commands. The latter
will also `forward` the state of the local blockchain to the latest block within the remotes.

```
cardano-cli blockchain remote-fetch testnet
```

_hint_: for fast download try `remote-fetch testnet hermes` first.

Now that you have downloaded the block from the remotes, you need to choose which of the new
blocks will be the next _tip_ of your local copy.

```
cardano-cli blockchain forward testnet a928cb61b01...
```

Or you can let the CLI choose for you the next tip by omitting the block hash parameter.

_hint_: use `pull` command to combine `remote-fetch` and `forward`.


### Blockchain status and exploration

You can list **all** the blocks of a given blockchain with the following command:

```
cardano-cli blockchain log testnet
```

Or you can specify a block hash to start listing the blocks of the blockchain from a given block
instead of the local tip.

To get information about a given block in particular, simply use the `cat` command:

```
cardano-cli blockchain cat testnet 87283e69ae2a245ff40e405706ac8ba5806c79d1914c2799a29f287652c4e93c
```

## `wallet`commands

Here lie all the operations relating to wallet management. The wallet is independent from
the blockchain itself. It contains only the cryptographic material of the wallet as well
as some details and flavors of the wallet (derivation's scheme, HD scheme etc...).
For security reasons the cryptographic materials are stored encrypted. The password to
store the cryptographic material is called a **spending password**. It is recommended
to set one.

### Creating a new wallet

It is possible to create a new wallet with `cardano-cli`. The command is `create`
and it takes the name of the wallet you want to create.

```
cardano-cli wallet create WalletName
```

You will be prompted to enter 2 kinds of password:

1. the first one is the password associated to the mnemonics of your wallet. Meaning
   that you will **_always_** need this secret associated to your mnemonics to recover
   the wallet.
2. the second one is the spending password. More generally it is the password that
   we use to encrypt your private key in the persistent data of the wallet.
   This password does not affect the recovering of the wallet. If you lose it you can
   still recover your wallet with the mnemonics (and the mnemonics' password if any).

### Recovering a wallet

You can recover a wallet using the command `recover`. By default it will recover
a wallet created via `cardano-cli`. The command will ask you the mnemonic phrase
or you can have a more interactive experience with the `--interactive` option where
the mnemonics will be asked one by one.

#### Recovering a Daedalus wallet

For various reasons the CLI does not allow to create Daedalus compatible wallet.
But it is possible to recover a wallet created on the Daedalus application.

```
cardano-cli wallet recover --daedalus-seed --derivation-scheme=v1 --mnemonics-length=12 --wallet-scheme=random_index_2levels MyDaedalusWallet
```

And that's it, you will your daedalus wallet recovered here. No need to transfer
the funds to another address yet (and therefor save some transaction fees).

#### Recovering an Icarus wallet

To recover an Icarus wallet, that's easy, only set the mnemonic length to 15.
By default Icarus wallets are using the same security settings as the `cardano-cli`.

**Reminder**: icarus wallets do not use mnemonics password, leave it empty.

```
cardano-cli wallet recover --mnemonics-length=15 MyIcarusWallet
```

### Recovering wallet's funds

By default a wallet is created standalone (i.e. not linked to a specific blockchain).
You need to attach the wallet to a blockchain.

```
cardano-cli wallet attach MyWallet BlockchainName
```

Now you can sync your wallet against the blockchain (i.e. recovering the transaction
histories and the available funds (UTxOs)). The `sync` command will analyse all the
blocks of the attached blockchain and will recover the owned addresses. The `sync`
will stop when the _local tip_ of the blockchain will be reached. If you don't see
recent addresses or transaction, remember to use the `blockchain pull` command.

```
cardano-cli wallet sync MyWallet
```

Depending of the blockchain size, the density of the transactions and the
hardware, this command may take some time.

Now you can list your wallet history, list the available funds or see a more general
status of the wallet.

```
cardano-cli wallet statement MyWallet
cardano-cli wallet utxos MyWallet
cardano-cli wallet status MyWallet
```

## `transaction` build engine

`cardano-cli` provides a simple yet powerful transaction build engine.
The transaction command is detached from the wallet. The wallet is needed
only for one ~tiny~ part of the transaction creation (the signing part).
All the preparation or the sending of the transaction to the network
does not need a wallet. Actually it does not need wallets at all.

This model allows you to build transactions using funds from different wallets.
You can then:

* split the bills (asks participants to commit funds to the transactions);
* allow your accountants to prepare the cheque for you to sign later;
* ...

### creating a new transaction

When creating a new transaction, you need to specify the blockchain
you will base your transaction upon. The command will return a unique
transaction identifier (called a *Staging Id*).

```bash
STAGING_ID=$(cardano-cli transaction new staging)
```

### Add outputs

Simply adds the addresses you want to send Ada and specify the amount
to send (**IN LOVELACE**).

```bash
cardano-cli transaction add-output ${STAGING_ID} ${ADDRESS} ${VALUE}
```

### Add a change address

this is the address that will be used to send the left over Ada
when finalizing the transaction. We currently only support adding
**1** change address. But this will change soon.

```bash
cardano-cli transaction add-change ${STAGING_ID} ${MY_CHANGE_ADDRESS}
```

### Adding inputs to the transaction

There are 2 methods to add inputs to a given transaction, either add it manually:

```bash
# this command list all the available inputs for this wallet
cardano-cli wallet utxos ${MY_WALLET_ALIAS}

# then add you inputs as follow:
cardano-cli transaction ${STAGING_ID} ${TxId} ${Index}
```

or use the ready to use input select:

```bash
cardano-cli transaction input-select ${STAGING_ID} ${WALLET_ALIAS1} ${WALLET_ALIAS2} # ...
```

This algorithm will select all the needed inputs from the given wallet(s).

There are 2 supported algorithms at the moment:

* `--select-largest-first` this algorithm will take the large inputs first. It is the default
  algorithm because it is the most likely to succeed to find enough inputs to make a transactions.
* `--select-exact-inputs=MAX_EXTRA_FEES` this one tries to select only the required inputs accepting
  to lose up to the given `MAX_EXTRA_FEES`.

### Finalizing the transaction

Finalizing the transaction does one useful action: It rebalances the outputs
and the change address. All the left over ada will be refunded to the given
change address.

```bash
cardano-cli transaction finalize ${STAGING_ID}
```

### Signing and Sending a transaction to the network

Now the transaction is ready, you can sign it then send it:

```bash
cardano-cli transaction sign ${STAGING_ID} ${MY_WALLET}
```

and to send it, nothing more easy

> **Be careful though**: once it is committed to the blockchain
> the transaction is not reversible. Check the transaction status
> before sending it.

```bash
cardano-cli transaction status ${STAGING_ID} ${MY_WALLET}
cardano-cli transaction send ${STAGING_ID} staging
```

# Commands documentation

## `blockchain`

### `blockchain cat`

print the content of a block.

USAGE:

    cardano-cli blockchain cat [FLAGS] <BLOCKCHAIN_NAME> <HASH>

FLAGS:

        --no-parse    don't parse the block, flush the bytes direct to the standard output (not subject to `--quiet' option)
        --debug       dump the block in debug format

ARGS:

    <BLOCKCHAIN_NAME>    the blockchain name
    <HASH>               The block hash to open.

### `blockchain destroy`

destroy the given blockchain, deleting all the blocks downloaded from the disk.

USAGE:

    cardano-cli blockchain destroy <BLOCKCHAIN_NAME>

ARGS:

    <BLOCKCHAIN_NAME>    the blockchain name

### `blockchain list`

list local blockchains

USAGE:

    cardano-cli blockchain list [FLAGS]

FLAGS:

    -l, --detailed    display some information regarding the remotes

### `blockchain log`

print the block, one by one, from the given block hash or the tip of the blockchain.

USAGE:

    cardano-cli blockchain log <BLOCKCHAIN_NAME> [HASH]

ARGS:
    <BLOCKCHAIN_NAME>    the blockchain name
    <HASH>               The hash to start from (instead of the local blockchain's tip).

### `blockchain new`

create a new local blockchain

USAGE:

    cardano-cli blockchain new [OPTIONS] <BLOCKCHAIN_NAME>

OPTIONS:

        --template <TEMPLATE>    the template for the new blockchain [default: mainnet]  [possible values: mainnet, staging, testnet]

ARGS:

    <BLOCKCHAIN_NAME>    the blockchain name

### `blockchain pull`

handy command to `remote-fetch` and `forward` the local blockchain.

USAGE:

    cardano-cli blockchain pull <BLOCKCHAIN_NAME>

ARGS:

    <BLOCKCHAIN_NAME>    the blockchain name

### `blockchain remote-add`

Attach a remote node to the local blockchain, this will allow to sync the local blockchain with this remote node.

USAGE:

    cardano-cli blockchain remote-add <BLOCKCHAIN_NAME> <BLOCKCHAIN_REMOTE_ALIAS> <BLOCKCHAIN_REMOTE_ENDPOINT>

ARGS:

    <BLOCKCHAIN_NAME>               the blockchain name
    <BLOCKCHAIN_REMOTE_ALIAS>       Alias given to a remote node.
    <BLOCKCHAIN_REMOTE_ENDPOINT>    Remote end point (IPv4 or IPv6 address or domain name. May include a port number. And a sub-route point in case of an http endpoint.

### `blockchain remote-fetch`

Fetch blocks from the remote nodes (optionally specified by the aliases).

USAGE:

    cardano-cli blockchain remote-fetch <BLOCKCHAIN_NAME> [BLOCKCHAIN_REMOTE_ALIAS]...

ARGS:

    <BLOCKCHAIN_NAME>               the blockchain name
    <BLOCKCHAIN_REMOTE_ALIAS>...    Alias given to a remote node.

### `blockchain remote-ls`

List all the remote nodes of the given blockchain

USAGE:

    cardano-cli blockchain remote-ls [FLAGS] <BLOCKCHAIN_NAME>

FLAGS:

        --detailed    print all local known information regarding the remotes
        --complete    print all local known information regarding the remotes as well as the details from the remote (needs a network connection)
        --short       print only the bare minimum information regarding the remotes (default)

ARGS:

    <BLOCKCHAIN_NAME>    the blockchain name

### `blockchain remote-rm`

Remove the given remote node from the local blockchain, we will no longer fetch blocks from this remote node.

USAGE:

    cardano-cli blockchain remote-rm <BLOCKCHAIN_NAME> <BLOCKCHAIN_REMOTE_ALIAS>

ARGS:

    <BLOCKCHAIN_NAME>            the blockchain name
    <BLOCKCHAIN_REMOTE_ALIAS>    Alias given to a remote node.

### `blockchain status`

print some details about the given blockchain

USAGE:

    cardano-cli blockchain status <BLOCKCHAIN_NAME>

ARGS:

    <BLOCKCHAIN_NAME>    the blockchain name

### `blockchain verify`

verify all blocks in the chain

USAGE:

    cardano-cli blockchain verify <BLOCKCHAIN_NAME>

ARGS:

    <BLOCKCHAIN_NAME>    the blockchain name

### `blockchain verify-block`

verify the specified block

USAGE:

    cardano-cli blockchain verify-block <BLOCKCHAIN_NAME> <HASH>

ARGS:

    <BLOCKCHAIN_NAME>    the blockchain name
    <HASH>               The hash of the block to verify.

## `wallet`

### `wallet address`

create a new address

USAGE:

    cardano-cli wallet address [FLAGS] <WALLET_NAME> <ACCOUNT_INDEX> <ADDRESS_INDEX>

FLAGS:

        --internal

ARGS:

    <WALLET_NAME>      the wallet name
    <ACCOUNT_INDEX>
    <ADDRESS_INDEX>

### `wallet attach`

Attach the existing wallet to the existing local blockchain. Detach first to attach to an other blockchain.

USAGE:

    cardano-cli wallet attach <WALLET_NAME> <BLOCKCHAIN_NAME>

ARGS:
    <WALLET_NAME>        the wallet name
    <BLOCKCHAIN_NAME>    the blockchain name

### `wallet create`

create a new wallet

USAGE:

    cardano-cli wallet create [OPTIONS] <WALLET_NAME>

OPTIONS:

        --derivation-scheme <DERIVATION_SCHEME>       derivation scheme [default: v2]  [possible values: v1, v2]
        --mnemonics-languages <MNEMONIC_LANGUAGES>    the list of languages to display the mnemonic words of the wallet in. You can set multiple values using comma delimiter
                                                      (example: `--mnemonics-languages=english,french,italian'). [default: english]  [possible values: chinese-simplified,
                                                      chinese-traditional, english, french, italian, japanese, korean, spanish]
        --mnemonics-length <MNEMONIC_SIZE>            The number of words to use for the wallet mnemonic (the more the more secure). [default: 24]  [possible values: 12, 15,
                                                      18, 21, 24]
        --wallet-scheme <WALLET_SCHEME>               the scheme to organize accounts and addresses in a Wallet. [default: bip44]  [possible values: bip44,
                                                      random_index_2levels]

ARGS:

    <WALLET_NAME>    the wallet name

### `wallet destroy`

delete all data associated to the given wallet.

USAGE:

    cardano-cli wallet destroy <WALLET_NAME>

ARGS:

    <WALLET_NAME>    the wallet name

### `wallet detach`

detach the wallet from its associated blockchain

USAGE:

    cardano-cli wallet detach <WALLET_NAME>

ARGS:

    <WALLET_NAME>    the wallet name

### `wallet list`

list all the wallets available

USAGE:

    cardano-cli wallet list [FLAGS] [OPTIONS]

FLAGS:

    -l, --detailed    display some metadata information of the wallet

OPTIONS:

        --color <COLOR>    enable output colors or not [default: auto]  [possible values: auto, always, never]

### `wallet log`

print the wallet logs

USAGE:

    cardano-cli wallet log <WALLET_NAME>

ARGS:

    <WALLET_NAME>    the wallet name

### `wallet recover`

recover a wallet

USAGE:

    cardano-cli wallet recover [FLAGS] [OPTIONS] <WALLET_NAME>

FLAGS:

        --daedalus-seed    To recover a wallet generated from daedalus
    -i, --interactive      use interactive mode for recovering the mnemonic words

OPTIONS:

        --derivation-scheme <DERIVATION_SCHEME>     derivation scheme [default: v2]  [possible values: v1, v2]
        --mnemonics-language <MNEMONIC_LANGUAGE>    the language of the mnemonic words to recover the wallet from. [default: english]  [possible values: chinese-simplified,
                                                    chinese-traditional, english, french, italian, japanese, korean, spanish]
        --mnemonics-length <MNEMONIC_SIZE>          The number of words to use for the wallet mnemonic (the more the more secure). [default: 24]  [possible values: 12, 15, 18,
                                                    21, 24]
        --wallet-scheme <WALLET_SCHEME>             the scheme to organize accounts and addresses in a Wallet. [default: bip44]  [possible values: bip44, random_index_2levels]

ARGS:

    <WALLET_NAME>    the wallet name

### `wallet statement`

print the wallet statement

USAGE:

    cardano-cli wallet statement <WALLET_NAME>

ARGS:

    <WALLET_NAME>    the wallet name

### `wallet status`

print some status information from the given wallet (funds, transactions...)

USAGE:

    cardano-cli wallet status <WALLET_NAME>

ARGS:
    <WALLET_NAME>    the wallet name

### `wallet sync`

synchronize the wallet with the attached blockchain

USAGE:

    cardano-cli wallet sync [FLAGS] [OPTIONS] <WALLET_NAME>

FLAGS:

        --dry-run    perform the sync without storing the updated states.

OPTIONS:

        --to <HASH>        sync the wallet up to the given hash (otherwise, sync up to local blockchain's tip).

ARGS:

    <WALLET_NAME>    the wallet name


### `wallet utxos`

print the wallet's available funds

USAGE:

    cardano-cli wallet utxos <WALLET_NAME>

ARGS:

    <WALLET_NAME>    the wallet name

## `transaction`

### `transaction add-change`

Add a change address to a transaction

USAGE:

    cardano-cli transaction add-change <TRANSACTION_ID> <CHANGE_ADDRESS>

ARGS:

    <TRANSACTION_ID>    the transaction staging identifier
    <CHANGE_ADDRESS>    address to send the change to

### `transaction add-input`

Add an input to a transaction

USAGE:

    cardano-cli transaction add-input <TRANSACTION_ID> [ARGS]

ARGS:

    <TRANSACTION_ID>        the transaction staging identifier
    <TRANSACTION_TXID>      A Transaction identifier in hexadecimal
    <TRANSACTION_INDEX>     The index of the unspent output in the transaction
    <TRANSACTION_AMOUNT>    The value in lovelace

### `transaction add-output`

Add an output to a transaction

USAGE:

    cardano-cli transaction add-output <TRANSACTION_ID> [ARGS]

ARGS:

    <TRANSACTION_ID>         the transaction staging identifier
    <TRANSACTION_ADDRESS>    Address to send funds too
    <TRANSACTION_AMOUNT>     The value in lovelace

### `transaction destroy`

Destroy a staging transaction

USAGE:

    cardano-cli transaction destroy <TRANSACTION_ID>

ARGS:

    <TRANSACTION_ID>    the transaction staging identifier

### `transaction export`

Export a staging transaction for transfer into a human readable format

USAGE:

    cardano-cli transaction export <TRANSACTION_ID> [EXPORT_FILE]

ARGS:

    <TRANSACTION_ID>    the transaction staging identifier
    <EXPORT_FILE>       optional file to export the staging transaction to (default will display the export to stdout)

### `transaction finalize`

Finalize a staging transaction

USAGE:

    cardano-cli transaction finalize <TRANSACTION_ID>

ARGS:

    <TRANSACTION_ID>    the transaction staging identifier

### `transaction import`

Import a human readable format transaction into a new staging transaction

USAGE:

    cardano-cli transaction import [IMPORT_FILE]

ARGS:

    <IMPORT_FILE>    optional file to import the staging transaction from (default will read stdin)

### `transaction input-select`

Select input automatically using a wallet (or a set of wallets), and a input selection algorithm

USAGE:

    cardano-cli transaction input-select [FLAGS] [OPTIONS] <TRANSACTION_ID> <WALLET_NAME>...

FLAGS:

        --quiet                   run the command quietly, do not print anything to the command line output
        --select-largest-first    Order the input by size, take the largest ones first to build this transaction

OPTIONS:

        --select-exact-inputs <MAX_EXTRA_FEES>
            select the exact necessary amount to perform the transaction. The optional parameter takes the accepted loss
            (in Lovelace, 1µ Ada).

ARGS:

    <TRANSACTION_ID>    the transaction staging identifier
    <WALLET_NAME>...    wallet name to use for the selection

### `transaction list`

List all staging transactions open

USAGE:

    cardano-cli transaction list

### `transaction new`

Create a new empty staging transaction

USAGE:

    cardano-cli transaction new <BLOCKCHAIN_NAME>

ARGS:

    <BLOCKCHAIN_NAME>    Transaction are linked to a blockchain to be valid

### `transaction rm-change`

Remove a change address from a transaction

USAGE:

    cardano-cli transaction rm-change <TRANSACTION_ID> <CHANGE_ADDRESS>

ARGS:

    <TRANSACTION_ID>    the transaction staging identifier
    <CHANGE_ADDRESS>    address to remove

### `transaction rm-input`

Remove an input to a transaction

USAGE:

    cardano-cli transaction rm-input <TRANSACTION_ID> [ARGS]

ARGS:

    <TRANSACTION_ID>       the transaction staging identifier
    <TRANSACTION_TXID>     A Transaction identifier in hexadecimal
    <TRANSACTION_INDEX>    The index of the unspent output in the transaction

### `transaction rm-output`

Remove an output to a transaction

USAGE:

    cardano-cli transaction rm-output <TRANSACTION_ID> [TRANSACTION_ADDRESS]

ARGS:

    <TRANSACTION_ID>         the transaction staging identifier
    <TRANSACTION_ADDRESS>    Address to send funds too

### `transaction send`

Send the transaction transaction to the blockchain

USAGE:

    cardano-cli transaction send <TRANSACTION_ID> <BLOCKCHAIN_NAME>

ARGS:

    <TRANSACTION_ID>     the transaction staging identifier
    <BLOCKCHAIN_NAME>    The blockchain the send the transaction too (will contact the peers of this blockchain)

### `transaction sign`

Finalize a staging a transaction into a transaction ready to send to the blockchain network

USAGE:

    cardano-cli transaction sign <TRANSACTION_ID>

ARGS:

    <TRANSACTION_ID>    the transaction staging identifier

### `transaction status`

Status of a staging transaction

USAGE:

    cardano-cli transaction status <TRANSACTION_ID>

ARGS:

    <TRANSACTION_ID>    the transaction staging identifier

## `debug`

### `debug address`

check if the given address (in base58) is valid and print information about it.

USAGE:

    cardano-cli debug address <ADDRESS>

ARGS:

    <ADDRESS>    base58 encoded address
