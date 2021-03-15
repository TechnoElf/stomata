let
  moz_overlay = import (builtins.fetchTarball https://github.com/mozilla/nixpkgs-mozilla/archive/master.tar.gz);
  pkgs = import <nixpkgs> {
    overlays = [ moz_overlay ];
  };
  cross_pkgs = import <nixpkgs> {
    overlays = [ moz_overlay ];
    crossSystem = {
      config = "aarch64-unknown-linux-gnu";
    };
  };
in with pkgs; pkgs.mkShell {
  buildInputs = [
    (rustChannelOfTargets "nightly" "2021-03-13" [ "x86_64-unknown-linux-gnu" "aarch64-unknown-linux-gnu" ])
    cross_pkgs.stdenv.cc
  ];
}
