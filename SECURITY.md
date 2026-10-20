# Security

SbarHive processes files only on your machine. It opens no network
connections and its only listening socket is a Unix socket in
`$XDG_RUNTIME_DIR`, readable by your user alone.

## Reporting a vulnerability

Please report security problems privately through GitHub's
"Report a vulnerability" button on the repository's Security tab, not in a
public issue. Include the version (`sbarhive --version`), your desktop and
distribution, and a sample file if the problem is triggered by one.

Of particular interest: crafted input files that crash an engine or write
outside the output folder, and anything that lets another local user talk to
the daemon socket.

## Supported versions

Only the latest release receives fixes.
