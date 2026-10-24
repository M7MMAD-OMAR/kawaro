# Update fixtures (S-053b)

Throwaway test material for `crates/core/tests/s053b_update_fetch.rs`. Nothing here is
a real release key.

- `key-c.pub`: public half of a throwaway minisign key pair made with
  `minisign -G -W`. The secret half was deleted after signing.
- `manifest-*.json` with a `.minisig` by key C: duplicate assets (the first rpm
  wins), schema 2, a beta manifest, a manifest without a linux asset, and
  `manifest-url-*` with one bad asset url each (`..`, `%2e`, `%2E`, backslash,
  space, control character, tab) on the first asset, a msi.
- The payloads behind the sizes and hashes are `first-asset-bytes` (17 bytes) and
  `second-asset-bytes` (18 bytes).
