# S-054b fixtures

Throwaway material for `crates/daemon/src/update_tests.rs`. `key-d.pub` is the public half of a throwaway minisign key (made with `minisign -G -W`; the secret half was shredded after signing). `manifest-stable-0.4.0.json` lists rpm, deb and msi assets of 17 bytes with the sha256 of `first-asset-bytes`; its `.minisig` is signed by key D. Not a real release key.
