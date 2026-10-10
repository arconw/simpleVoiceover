#!/usr/bin/env bash
set -euo pipefail
audio="${1:-pulseaudio}"
codecs="${2:-full}"
webkit_version="${3:-}"
source /etc/os-release
case "$audio" in pulseaudio|pipewire) ;; *) exit 2 ;; esac
case "$codecs" in minimal|full) ;; *) exit 2 ;; esac
case "$ID" in
  ubuntu|debian)
    export DEBIAN_FRONTEND=noninteractive
    apt-get update
    packages=(ca-certificates python3 dbus-x11 xvfb openbox weston x11vnc novnc websockify xterm ffmpeg fonts-dejavu-core procps binutils libwebkit2gtk-4.1-0 libjavascriptcoregtk-4.1-0 webkit2gtk-driver libpulse0 pulseaudio-utils gstreamer1.0-tools gstreamer1.0-plugins-base gstreamer1.0-plugins-good xdg-desktop-portal xdg-desktop-portal-gtk)
    if [[ "$audio" == pulseaudio ]]; then
      packages+=(pulseaudio)
    else
      packages+=(pipewire pipewire-pulse wireplumber)
    fi
    if [[ "$codecs" == full ]]; then packages+=(gstreamer1.0-libav gstreamer1.0-plugins-bad); fi
    if [[ -n "$webkit_version" ]]; then
      packages+=("libwebkit2gtk-4.1-0=$webkit_version" "libjavascriptcoregtk-4.1-0=$webkit_version" "webkit2gtk-driver=$webkit_version")
    fi
    apt-get install --yes --no-install-recommends "${packages[@]}"
    rm -rf /var/lib/apt/lists/*
    ;;
  fedora)
    packages=(ca-certificates python3 dbus-daemon xorg-x11-server-Xvfb openbox weston x11vnc novnc python3-websockify xterm dejavu-sans-fonts procps-ng binutils webkit2gtk4.1 javascriptcoregtk4.1 webkitgtk6.0 pulseaudio-utils gstreamer1 gstreamer1-plugins-base gstreamer1-plugins-good xdg-desktop-portal xdg-desktop-portal-gtk ffmpeg-free)
    if [[ "$audio" == pulseaudio ]]; then packages+=(pulseaudio); else packages+=(pipewire pipewire-pulseaudio wireplumber); fi
    if [[ "$codecs" == full ]]; then packages+=(gstreamer1-plugins-bad-free gstreamer1-plugin-libav gstreamer1-plugin-openh264); fi
    if [[ -n "$webkit_version" ]]; then
      packages+=("webkit2gtk4.1-$webkit_version" "javascriptcoregtk4.1-$webkit_version" "webkitgtk6.0-$webkit_version")
    fi
    dnf install --assumeyes --setopt=install_weak_deps=False "${packages[@]}"
    dnf clean all
    ;;
  arch)
    packages=(ca-certificates curl python python-pip dbus xorg-server-xvfb openbox weston x11vnc xterm ttf-dejavu procps-ng binutils webkit2gtk-4.1 webkitgtk-6.0 libpulse gst-plugins-base gst-plugins-good xdg-desktop-portal xdg-desktop-portal-gtk ffmpeg)
    if [[ "$audio" == pulseaudio ]]; then packages+=(pulseaudio); else packages+=(pipewire pipewire-pulse wireplumber); fi
    if [[ "$codecs" == full ]]; then packages+=(gst-libav gst-plugins-bad); fi
    if [[ -n "$webkit_version" ]]; then
      printf '%s\n' 'Arch WebKit pinning requires an Arch Linux Archive base image/repository snapshot; partial upgrades are not supported.' >&2
      exit 2
    fi
    pacman --sync --refresh --sysupgrade --noconfirm --needed "${packages[@]}"
    python3 -m venv /opt/vnc-python
    /opt/vnc-python/bin/pip install --no-cache-dir websockify==0.13.0
    ln -s /opt/vnc-python/bin/websockify /usr/local/bin/websockify
    curl --fail --location --retry 3 https://github.com/novnc/noVNC/archive/refs/tags/v1.7.0.tar.gz --output /tmp/novnc.tar.gz
    printf '%s\n' 'b1003a11b6e6e8d8f7f5e5586daae7f8ca651d8aee0aa155ff9ac841c48f52c6  /tmp/novnc.tar.gz' | sha256sum --check
    mkdir -p /usr/share/novnc
    tar --extract --gzip --file=/tmp/novnc.tar.gz --strip-components=1 --directory=/usr/share/novnc
    rm /tmp/novnc.tar.gz
    pacman --sync --clean --clean --noconfirm
    ;;
  *) printf 'Unsupported distribution: %s\n' "$ID" >&2; exit 2 ;;
esac
