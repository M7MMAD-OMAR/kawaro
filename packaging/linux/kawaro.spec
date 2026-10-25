# SPDX-License-Identifier: GPL-3.0-or-later
# Binary RPM from a tree made by packaging/linux/stage.sh.
# Built by packaging/linux/build-packages.sh, which passes version and stage.
Name:           kawaro
Version:        %{pkg_version}
Release:        1
Summary:        Local file converter with a honeycomb picker in your file manager
License:        GPL-3.0-or-later
URL:            https://github.com/M7MMAD-OMAR/kawaro
ExclusiveArch:  x86_64
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

%install
cp -a %{stage}/. %{buildroot}/

%post
systemctl --global enable kawaro.service >/dev/null 2>&1 || :

%preun
if [ "$1" -eq 0 ]; then
  systemctl --global disable kawaro.service >/dev/null 2>&1 || :
fi

%changelog
* Tue Oct 20 2026 MUHMAD HAJ HMDO - see CHANGELOG.md
- Packaged from the upstream release

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
/usr/share/kio/servicemenus/kawaro.desktop
/usr/share/nautilus-python/extensions/kawaro.py
/usr/share/nemo/actions/kawaro.nemo_action
