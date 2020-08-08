let
  sources = import ./nix/sources.nix;
  rust = import ./nix/rust.nix { inherit sources; };
  pkgs = import sources.nixpkgs { crossSystem = {
    config = "x86_64-pc-windows-gnu";
  }; };
in
pkgs.mkShell {
  buildInputs = with pkgs; [
    yarn
    nodejs
    rust
    pkgconfig
    openssl
    gcc
    adoptopenjdk-bin
    pkgsCross.mingwW64
  ];
}
