# SPDX-License-Identifier: GPL-3.0-or-later
# Source RPM for Fedora COPR (S-103d, S-016c). Builds kawaro and kawarod from
# the tagged source with the locked, vendored Cargo dependencies and no
# network, then lays out the same files as the release RPM
# (packaging/linux/kawaro.spec) through packaging/linux/stage.sh.
#
# scripts/channels/copr.sh makes the two sources and passes the version:
#   Source0  git archive of the tag, prefix kawaro-VERSION/
#   Source1  cargo vendor output in vendor-crates/ (not vendor/, which holds
#            the in tree softbuffer patch)
%{!?kawaro_version:%global kawaro_version 0.0.0}
Name:           kawaro
Version:        %{kawaro_version}
Release:        1%{?dist}
Summary:        Local file converter with a honeycomb picker in your file manager
# The licences of the vendored crates are listed in the source tree's
# Cargo.lock; the project itself is GPL-3.0-or-later.
License:        GPL-3.0-or-later
URL:            https://github.com/M7MMAD-OMAR/kawaro
Source0:        %{name}-%{version}.tar.gz
Source1:        %{name}-%{version}-vendor.tar.xz
ExclusiveArch:  x86_64
BuildRequires:  cargo >= 1.85
BuildRequires:  rust >= 1.85
BuildRequires:  gcc
BuildRequires:  pkgconfig(fontconfig)
BuildRequires:  pkgconfig(xkbcommon)
BuildRequires:  pkgconfig(wayland-client)
Recommends:     (ffmpeg-free or ffmpeg)
Recommends:     ghostscript
Recommends:     (libreoffice-writer or libreoffice-core)
Recommends:     wl-clipboard
Recommends:     curl
Suggests:       nautilus-python

%global debug_package %{nil}
%global __strip /bin/true

%description
Kawaro converts and processes files entirely on your machine. Select files
in Dolphin, Nautilus or Nemo, choose Kawaro from the right-click menu, and
pick an action from the hexagonal hive. The result is saved next to the
original, which is never modified. Arabic and English.

%prep
%autosetup -n %{name}-%{version} -a 1
mkdir -p .cargo
# Appended: the tree's own .cargo/config.toml must keep its [env] table.
cat >> .cargo/config.toml <<'CONFIG'

[source.crates-io]
replace-with = "vendored-sources"

[source.vendored-sources]
directory = "vendor-crates"
CONFIG

%build
cargo build --release --locked --offline -p kawaro-cli -p kawaro-daemon

%install
packaging/linux/stage.sh target/release %{buildroot}

%post
systemctl --global enable kawaro.service >/dev/null 2>&1 || :

%preun
if [ "$1" -eq 0 ]; then
  systemctl --global disable kawaro.service >/dev/null 2>&1 || :
fi

%changelog
* Fri Oct 30 2026 Kawaro maintainers - see CHANGELOG.md
- Built in COPR from the upstream release tag

%files
%license /usr/share/licenses/kawaro/LICENSE
%doc /usr/share/doc/kawaro
/usr/bin/kawaro
/usr/bin/kawarod
/usr/bin/kawaro-open-selection
/usr/lib/systemd/user/kawaro.service
/usr/share/applications/io.github.m7mmad_omar.Kawaro.desktop
/usr/share/metainfo/io.github.m7mmad_omar.Kawaro.metainfo.xml
/usr/share/icons/hicolor/*/apps/kawaro*
/usr/share/kawaro/screenshots
/usr/share/kio/servicemenus/kawaro.desktop
/usr/share/nautilus-python/extensions/kawaro.py
/usr/share/nemo/actions/kawaro.nemo_action
