let
  nixpkgs = fetchTarball "https://github.com/NixOS/nixpkgs/tarball/nixos-24.11";
  pkgs = import nixpkgs {
    config = { };
    overlays = [ ];
  };
in

pkgs.mkShellNoCC {
  packages = with pkgs; [
    pkg-config
    gnumake
    gcc
    libiconv
    autoconf
    automake
    libtool
    cmake
    alsa-lib
    alsa-lib.dev
    alsa-utils
    udev.dev
    wayland
    libxkbcommon
    webkitgtk_4_1
    libsoup_3
    gtk3
    gst_all_1.gstreamer
    gst_all_1.gst-plugins-base
    gst_all_1.gst-plugins-good
    gst_all_1.gst-plugins-bad
  ];

  shellHook = ''
    export LD_LIBRARY_PATH=${pkgs.wayland}/lib:$LD_LIBRARY_PATH
    export LD_LIBRARY_PATH=${pkgs.libxkbcommon}/lib:$LD_LIBRARY_PATH
    export LD_LIBRARY_PATH=${pkgs.amdvlk}/lib:$LD_LIBRARY_PATH
    export LD_LIBRARY_PATH=${pkgs.webkitgtk_4_1}/lib:$LD_LIBRARY_PATH
    export RUSTFLAGS="$RUSTFLAGS -C link-arg=-Wl,-rpath,"
    export RUSTFLAGS="$RUSTFLAGS:${pkgs.wayland}/lib"
    export RUSTFLAGS="$RUSTFLAGS:${pkgs.libxkbcommon}/lib"
    export RUSTFLAGS="$RUSTFLAGS:${pkgs.webkitgtk_4_1}/lib"
  '';

}
