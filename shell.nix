let
  moz_overlay = import (builtins.fetchTarball https://github.com/mozilla/nixpkgs-mozilla/archive/master.tar.gz);
  pkgs = import <nixpkgs> {
    overlays = [ moz_overlay ];
  };
in with pkgs; pkgs.mkShell {
  buildInputs = [
    (rustChannelOfTargets "nightly" "2021-05-04" [ "x86_64-unknown-linux-gnu" ])
  ];
}
