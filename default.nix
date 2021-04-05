let
  moz_overlay = import (builtins.fetchTarball https://github.com/mozilla/nixpkgs-mozilla/archive/master.tar.gz);
  pkgs = import <nixpkgs> {
    overlays = [ moz_overlay ];
  };
  cross_pkgs = import <nixpkgs> {
    overlays = [ moz_overlay ];
    crossSystem = {
      config = "aarch64-unknown-linux-gnu";
      rustc.config = "aarch64-unknown-linux-gnu";
    };
  };
  rust = (cross_pkgs.rustChannelOfTargets "nightly" "2021-03-13" [ "aarch64-unknown-linux-gnu" ]);
  rustPlatform = cross_pkgs.makeRustPlatform {
    cargo = rust;
    rustc = rust;
  };
in with pkgs; rustPlatform.buildRustPackage rec {
  pname = "stomata";
  version = "0.1.0";

  src = ./.;

  cargoSha256 = "07n5dh19l527izkpf0g90iz70mf8n2c9khk7dsm00zpi8x6kmdav";

  buildInputs = [
    cross_pkgs.stdenv.cc
  ];

  preConfigure = ''
    export HOME=$(mktemp -d)
  '';
}
