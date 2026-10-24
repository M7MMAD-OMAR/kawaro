#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# Runs the S-053c release tests. Needs minisign, jq, shellcheck, python3 with
# PyYAML, rpmbuild. Light: no compile. The Rust half is
# crates/core/tests/s053c_update_feed.rs.
set -u
HERE="$(cd "$(dirname "$0")" && pwd)"
RC=0
for t in manifest sign workflow packages docs beta_versions beta_manifest beta_workflow beta_notes beta_dryrun; do
  echo "== $t"
  "$HERE/$t.sh" || RC=1
done
exit "$RC"
