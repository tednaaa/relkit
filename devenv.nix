{ pkgs, ... }:
{
  languages = {
    rust.enable = true;
  };

  packages = [ pkgs.lefthook ];

  enterShell = "lefthook install";
}
