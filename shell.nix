let
  sources = import ./nix/sources.nix;
  rust = import ./nix/rust.nix { inherit sources; };
  pkgs = import sources.nixpkgs {};
in
pkgs.mkShell {
  buildInputs = with pkgs; [
    yarn
    nodejs
    rust
    openjdk
    pkgconfig
    openssl
    gcc
    jdk
  ];
}
