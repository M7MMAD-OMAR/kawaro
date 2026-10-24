# Update fixtures (S-053a)

Throwaway test material for `crates/core/tests/s053a_update.rs`. Nothing here is a
real release key.

- `key-a.pub`, `key-b.pub`: public halves of two throwaway minisign key pairs. The
  secret halves were never stored; they were deleted after signing.
- `manifest-stable-0.4.0.json`: a valid schema 1 manifest with three assets.
- `manifest-stable-0.4.0.json.minisig`: its signature by key A.
- `manifest-stable-0.4.0.json.b.minisig`: its signature by key B.
- `manifest-*.json` (the others): one defect each (schema, sha256, size, kind,
  invalid JSON) or an unknown key that must be ignored.

Made once with `minisign -G -W` and `minisign -S`. Tests never run minisign.
