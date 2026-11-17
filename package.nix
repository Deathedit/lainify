# `nix build` / `nix run`: a standalone lainify binary that doesn't need the dev shell.
{
  lib,
  rustPlatform,
  qt6,
  gst_all_1,
  pkg-config,
  libglvnd,
}:

let
  qtModules = [
    qt6.qtbase
    qt6.qtdeclarative
    qt6.qtmultimedia
    qt6.qtsvg # icons
  ];
  # cxx-qt-build needs one qmake that sees every module; the env tree provides it.
  qtEnv = qt6.env "qt-lainify-${qt6.qtbase.version}" qtModules;

  gst = with gst_all_1; [
    gstreamer
    gst-plugins-base # videoconvert, appsink
    gst-plugins-good # v4l2src, jpegdec, pngenc, videoflip, pulsesrc/sink, level, matroskamux, mp4mux
    gst-plugins-bad # nvcodec (NVENC), h265parse, av1parse
    gst-plugins-ugly # x264enc
    gst-libav # avenc_aac
  ];
in
rustPlatform.buildRustPackage {
  pname = "lainify";
  version = (lib.importTOML ./Cargo.toml).package.version;

  src = lib.fileset.toSource {
    root = ./.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./build.rs
      ./src
      ./cpp
      ./qml
      ./assets
    ];
  };
  cargoLock.lockFile = ./Cargo.lock;

  nativeBuildInputs = [
    qtEnv
    pkg-config
    qt6.wrapQtAppsHook
  ];
  # The modules themselves (not qtEnv), so wrapQtAppsHook learns their plugin and QML paths
  # without baking the -dev env tree into the wrapper.
  buildInputs = qtModules ++ gst ++ [ libglvnd ];

  # Exported here rather than via `env`: qtbase's setup hook sets QMAKE itself.
  preBuild = ''
    export QMAKE="${qtEnv}/bin/qmake"
  '';

  # The desktop entry is what the desktop portal and launchers look the app ID up in.
  postInstall = ''
    install -Dm644 assets/lainify.desktop $out/share/applications/lainify.desktop
    install -Dm644 assets/lainify.svg $out/share/icons/hicolor/scalable/apps/lainify.svg
  '';

  # Linking follows $QMAKE, so RUNPATH would point at qtEnv and drag its closure in.
  preFixup = ''
    patchelf --set-rpath \
      "$(patchelf --print-rpath $out/bin/lainify \
         | sed 's|${qtEnv}/lib|${lib.makeLibraryPath qtModules}|')" \
      $out/bin/lainify
  '';

  qtWrapperArgs = [
    "--prefix"
    "GST_PLUGIN_SYSTEM_PATH_1_0"
    ":"
    # getLib: gstreamer's default output is `bin`; the plugins live in `out`.
    (lib.makeSearchPath "lib/gstreamer-1.0" (map lib.getLib gst))
  ];

  meta = {
    description = "Low-latency capture card viewer with uncompressed RGB 4:4:4 support";
    platforms = lib.platforms.linux;
    mainProgram = "lainify";
  };
}
