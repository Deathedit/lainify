# Dev environment for lainify — used by `nix-shell` directly and by the flake's devShell.
# Without arguments it pins nixpkgs to the revision in flake.lock, so both entry points
# get identical tool versions.
{
  pkgs ?
    let
      lock = (builtins.fromJSON (builtins.readFile ./flake.lock)).nodes.nixpkgs.locked;
    in
    import (fetchTarball {
      url = "https://github.com/${lock.owner}/${lock.repo}/archive/${lock.rev}.tar.gz";
      sha256 = lock.narHash;
    }) { },
}:

pkgs.mkShell {
  packages = with pkgs; [
    nodejs_22
    pnpm
    v4l-utils
  ];

  shellHook = ''
    echo "lainify dev shell — node $(node -v), pnpm $(pnpm -v)"
    echo "  ./run.sh          start the dev server"
    echo "  ./run.sh help     all commands"
  '';
}
