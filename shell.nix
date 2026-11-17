# Dev environment for lainify (Rust + Qt/QML + GStreamer), used by `nix-shell` and the flake.
# Without arguments it pins nixpkgs to the revision in flake.lock, so both entry points match.
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

let
  # qtmultimedia provides VideoOutput / QVideoSink, which the frames are drawn through.
  qtEnv = pkgs.qt6.env "qt-lainify-${pkgs.qt6.qtbase.version}" [
    pkgs.qt6.qtbase
    pkgs.qt6.qtdeclarative
    pkgs.qt6.qtmultimedia
  ];

  gst = with pkgs.gst_all_1; [
    gstreamer
    gst-plugins-base # videoconvert, appsink
    gst-plugins-good # v4l2src, jpegdec, pngenc
  ];
in
pkgs.mkShell {
  nativeBuildInputs = with pkgs; [
    cargo
    rustc
    clippy
    rustfmt
    rust-analyzer
    pkg-config
    qtEnv
    v4l-utils
  ];
  buildInputs = [ qtEnv pkgs.libglvnd ] ++ gst;

  # cxx-qt-build locates Qt through this qmake; the running app needs Qt's plugin and QML paths.
  QMAKE = "${qtEnv}/bin/qmake";
  QT_PLUGIN_PATH = "${qtEnv}/${pkgs.qt6.qtbase.qtPluginPrefix}";
  QML_IMPORT_PATH = "${qtEnv}/${pkgs.qt6.qtbase.qtQmlPrefix}";

  shellHook = ''
    echo "lainify dev shell — $(rustc --version), Qt ${pkgs.qt6.qtbase.version}, GStreamer ${pkgs.gst_all_1.gstreamer.version}"
    echo "  ./run.sh          build and start lainify"
    echo "  ./run.sh help     all commands"
  '';
}
