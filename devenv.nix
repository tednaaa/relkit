{ pkgs, ... }:
{
  languages = {
    rust = {
      enable = true;
      toolchainFile = ./rust-toolchain.toml;
    };
  };

  packages = [ pkgs.lefthook ];

  enterShell = "lefthook install";
}
