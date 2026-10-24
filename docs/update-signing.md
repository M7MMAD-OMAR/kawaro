# Update signing

How the SbarHive update feed is signed (ADR 0006, `docs/adr/0006-auto-update.md`).
Every release publishes a manifest (`stable.json`, `beta.json`) with a minisign
signature, and the app only trusts a manifest signed by the key committed in
`packaging/keys/sbarhive-update.pub`. Its key id is `FBABD7AD2ECE1452`.

## Generate the key

Do this once, on a real machine you control, never in CI:

```
minisign -G -p sbarhive-update.pub -s sbarhive-update.key
```

Choose a strong passphrase when asked. Commit only the `.pub` file, as
`packaging/keys/sbarhive-update.pub`. The key id is on its first line.

## Where the private key lives

The private key file stays offline, outside the repository, with a backup on
separate storage. It is never committed, pasted into an issue, or printed in a
log. The S-053c tests check that no secret key reaches git.

## Secrets and environment

The release job reads two GitHub secrets:

- `UPDATE_SIGNING_KEY`: the full content of the private key file.
- `UPDATE_SIGNING_PASSWORD`: its passphrase.

Both are stored in the protected environment `release-signing` of
`M7MMAD-OMAR/sbarhive`, so only the `update-manifest` job of a tag build in that
repository can read them, never a pull request or a fork. The job writes the
key to a file with `umask 077`, signs with `scripts/sign-update-manifest.sh`
(the passphrase goes to minisign on standard input) and deletes the file.

## Rotation

To rotate, generate a new pair, commit the new public key as
`packaging/keys/sbarhive-update-next.pub` and ship a normal release: the app
accepts a signature from either key. Once most users run that release, switch
the secrets to the new key, replace `sbarhive-update.pub` with the new public
key and remove the `next` file in a later release.

## If the key leaks

If the private key or its passphrase leaks, the attacker holds the same
signing power as the project, so no in-app recovery is possible: a forged
update would be accepted. Users must reinstall by hand from a fresh download
of a release signed by a new key. Revoke the secrets at once, generate a new
pair, publish a release with the new public key and announce it. This is the
limit accepted in ADR 0006.
