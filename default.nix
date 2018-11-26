{ nixpkgs ? fetchTarball channel:nixos-unstable
, pkgs ? import nixpkgs {}
}:

with pkgs;

rustPlatform.buildRustPackage {
  name = "cardano-cli";

  src = ./.;

  cargoSha256 = "1999p3h15q8sgmc4bs70ljq4s9s1b4k6yk5xkard9kn0sv469hhy";
}
