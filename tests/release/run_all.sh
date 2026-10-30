#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# Runs the S-053c release tests. Needs minisign, jq, shellcheck, python3 with
# PyYAML, rpmbuild. Light: no compile. The Rust half is
# crates/core/tests/s053c_update_feed.rs.
set -u
HERE="$(cd "$(dirname "$0")" && pwd)"
RC=0
for t in manifest sign workflow packages docs beta_versions beta_manifest beta_workflow beta_notes beta_dryrun s073_asset_names s083_windows_signing s078_winget_versions s078_winget_scope s100_msi_upgrade_works notes_manifest s103a_channels s103a_workflow s103b_winget s103c_scoop s103e_aur s103d_copr s103g_flathub; do
  echo "== $t"
  "$HERE/$t.sh" || RC=1
done
exit "$RC"
