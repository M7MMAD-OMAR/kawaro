"""Reference behaviour for ADR 0010 Part A: resumable, pinned, verified download.

Same primitive as crates/core/src/update/fetch.rs: the system curl, spawned with
a fixed argument list. Production is https only. Tests may pass test=True,
which allows plain http but only to 127.0.0.1 (the one difference).
"""
import hashlib
import os
import shutil
import subprocess
import time
from urllib.parse import urlsplit

MARGIN = 256 * 1024 * 1024  # free space kept beyond the file itself
RETRYABLE = {18, 28, 52, 55, 56}  # partial file, timeout, empty reply, send/recv error


def build_args(url, part, size, test=False, speed_limit=1024, speed_time=30,
               use_max_filesize=True, max_redirs=5):
    """Production flag set (fetch.rs flags, plus resume and progress timeout)."""
    protos = "=http,https" if test else "=https"
    args = ["--disable", "--silent", "--show-error", "--fail", "--location",
            "--proto", protos, "--proto-redir", "=https",
            "--max-redirs", str(max_redirs),
            "--connect-timeout", "30",
            "--speed-limit", str(speed_limit), "--speed-time", str(speed_time),
            "-C", "-"]
    if test:
        # Test set: redirects may also stay on plain http (loopback only).
        args[args.index("--proto-redir") + 1] = "=http,https"
    if use_max_filesize:
        args += ["--max-filesize", str(size)]
    return args + ["--user-agent", "kawaro-models", "--output", part, url]


def check_space(directory, need, margin=MARGIN, usage=shutil.disk_usage):
    """True when `need` bytes plus `margin` fit. Call before starting curl."""
    return usage(directory).free >= need + margin


def sha256_of(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for block in iter(lambda: f.read(1 << 20), b""):
            h.update(block)
    return h.hexdigest()


def _run(args, part, size, on_progress, poll):
    """Run curl, poll the .part size, kill it when it grows past the pinned size."""
    p = subprocess.Popen(["curl", *args], stdout=subprocess.DEVNULL,
                         stderr=subprocess.PIPE, text=True)
    over = False
    while True:
        try:
            p.wait(timeout=poll)
            break
        except subprocess.TimeoutExpired:
            have = os.path.getsize(part) if os.path.exists(part) else 0
            on_progress(have, size)
            if have > size:
                over = True
                p.kill()
                p.wait()
                break
    err = p.stderr.read().strip()
    p.stderr.close()
    return p.returncode, err, over


def download(url, dest, size, sha256, report, on_progress=lambda h, t: None,
             test=False, poll=1.0, margin=MARGIN, usage=shutil.disk_usage,
             **flags):
    """One attempt. Returns "ok" or a reason. `report` is called once on failure.

    A kept .part means "resume on the next call": only after a retryable curl
    exit. Every verification failure deletes it.
    """
    if test:
        host = urlsplit(url).hostname
        assert host == "127.0.0.1", "test mode is loopback only"
    elif urlsplit(url).scheme != "https":
        return _fail(report, "not-https")
    part = dest + ".part"
    have = os.path.getsize(part) if os.path.exists(part) else 0
    if have > size:  # stale or hostile leftover
        os.remove(part)
        have = 0
    if not check_space(os.path.dirname(dest), size - have, margin, usage):
        return _fail(report, "no-space")
    if have < size:  # have == size: a previous run finished downloading, only verify
        rc, err, over = _run(build_args(url, part, size, test, **flags),
                             part, size, on_progress, poll)
        if over:
            os.remove(part)
            return _fail(report, "oversize")
        if rc != 0:
            if rc not in RETRYABLE and os.path.exists(part):
                os.remove(part)  # 22 http error, 33 no range, 63 too big, ...
            return _fail(report, f"curl-{rc}: {err}")
    if os.path.getsize(part) != size:
        os.remove(part)
        return _fail(report, "size-mismatch")
    if sha256_of(part) != sha256:
        os.remove(part)
        return _fail(report, "hash-mismatch")
    os.replace(part, dest)  # atomic: the final name exists only when verified
    return "ok"


def _fail(report, reason):
    report(reason)
    return reason
