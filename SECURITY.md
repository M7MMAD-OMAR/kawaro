# Security

Kawaro processes files only on your machine. It opens no network
connections. On Linux its only listening socket is a Unix socket in
`$XDG_RUNTIME_DIR`, readable by your user alone.

On Windows the daemon listens on a named pipe instead, named after your user
and session. Its access list grants your user alone, it rejects clients from
other machines (`PIPE_REJECT_REMOTE_CLIENTS`), and the client refuses to send
anything to a pipe whose server runs as another user.
Another local user who creates the pipe name first can stop the daemon from
starting (a local denial of service, tracked for S-045). The Windows build is
not released yet and these protections are not verified on a live Windows
system until S-052.


## Reporting a vulnerability

Please report security problems privately through GitHub's
"Report a vulnerability" button on the repository's Security tab, not in a
public issue. Include the version (`kawaro --version`), your desktop and
distribution, and a sample file if the problem is triggered by one.

Of particular interest: crafted input files that crash an engine or write
outside the output folder, and anything that lets another local user talk to
the daemon socket or pipe.

## Supported versions

Only the latest release receives fixes.
