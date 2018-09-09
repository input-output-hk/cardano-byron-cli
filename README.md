# cardano-cli

The [Cardano](https://www.cardano.org) command line interface:

* powerful blockchain manager: download, explore, verify, analyse;
* manage multiple wallets: daedalus', icarus' or other kind of wallets;
* flexible transaction build engine.

This command line interface is built upon the
[**Rust Cardano SDK**](https://github.com/input-output-hk/rust-cardano).

## Installation guide

While it is recommended to wait for official releases, it is also possible
to build the executable yourself:

1. [install rust toolchain](;https://www.rust-lang.org/en-US/install.html);
2. clone the project repository (with the dependencies)
   ```sh
   git clone git@git@github.com:input-output-hk/cardano-cli.git
   ```
3. build and install the binary:
   ```sh
   cd cardano-cli
   cargo install
   ```
4. enjoy

## Usage

### Quick start

```sh
$ cardano-cli blockchain new mainnet
$ cardano-cli blockchain pull mainnet
$ cardano-cli wallet create "My Wallet"
$ cardano-cli wallet attach "My Wallet" mainnet
$ cardano-cli wallet sync   "My Wallet"
$ cardano-cli wallet status "My Wallet"
```

### Complete documentation

[Command line documentation](./USAGE.md)

# License

This project is licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or
   http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or
   http://opensource.org/licenses/MIT)

at your option.
