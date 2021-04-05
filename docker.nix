let
  pkgs = import <nixpkgs> {};
  stomata = import ./default.nix;
  alpine = pkgs.dockerTools.pullImage {
    imageName = "alpine";
    imageDigest = "sha256:c45a1db6e04b73aad9e06b08f2de11ce8e92d894141b2e801615fa7a8f68314a";
    sha256 = "1vsa61ran4gpaq8bbc3cn3zwsv9v83xlzdkc0v2d3yw77k7y3jgs";
  };
in pkgs.dockerTools.buildImage {
  name = "registry.undertheprinter.com/stomata";
  tag = "latest";

  fromImage = alpine;

  runAsRoot = ''
    #!/bin/sh
    cp ${stomata}/bin/stomata /bin/.
  '';

  contents = [ stomata ];
  config = {
    Cmd = [ "/bin/stomata" ];
  };
}
