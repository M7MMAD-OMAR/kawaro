#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# Ubuntu CI only. Bound acquisition, never kill/retry an active dpkg install.
set -euo pipefail
apt_root="${APT_CONFIG_ROOT:-/etc/apt}"
if [[ "${CI:-}" != true ]]; then
    echo "MinGW provisioning is for Ubuntu CI only" >&2
    exit 1
fi
if [[ "$apt_root" == /etc/apt ]]; then
    # shellcheck source=/dev/null
    source /etc/os-release
    [[ "$ID" == ubuntu ]] || { echo "Ubuntu required" >&2; exit 1; }
fi

apt_options=(-o Acquire::Retries=2 -o Acquire::http::Timeout=30 -o Acquire::https::Timeout=30)
packages=(gcc-mingw-w64-x86-64 g++-mingw-w64-x86-64)
acquire() {
    sudo timeout --kill-after=10s 90s apt-get "${apt_options[@]}" update -o APT::Update::Error-Mode=any &&
        sudo timeout --kill-after=10s 240s apt-get "${apt_options[@]}" --download-only install -y --no-install-recommends "${packages[@]}"
}

canonical_sources() {
    # Fixtures are writable without sudo. Only the production root needs elevation.
    local python=(python3)
    if [[ "$apt_root" == /etc/apt && ! -w "$apt_root" ]]; then
        python=(sudo python3)
    fi
    "${python[@]}" - "$apt_root" <<'PY'
import re
import sys
from pathlib import Path
from urllib.parse import urlsplit

root = Path(sys.argv[1]).resolve()
archive = 'https://archive.ubuntu.com/ubuntu'
security = 'https://security.ubuntu.com/ubuntu'
planned = {}
mirror_usage = {}
seen = set()


def refuse(message):
    raise ValueError(message)


def load(path):
    # Preserve line endings and every byte outside the selected URI tokens.
    return path.read_bytes().decode('utf-8')


def canonical(uri, security_only):
    parsed = urlsplit(uri)
    if (parsed.scheme not in ('http', 'https') or parsed.netloc not in
            ('azure.archive.ubuntu.com', 'archive.ubuntu.com', 'security.ubuntu.com')
            or parsed.path.rstrip('/') != '/ubuntu' or parsed.query or parsed.fragment):
        refuse('Unrecognized Ubuntu mirror URI')
    if parsed.netloc == 'security.ubuntu.com' and not security_only:
        refuse('Security-only mirror used for archive suites')
    return security if security_only else archive


def mirror_file(uri, security_only):
    name = uri.removeprefix('mirror+').removeprefix('file:')
    path = Path(name).resolve()
    if not path.is_relative_to(root):
        refuse('Mirror file is outside the APT configuration root')
    text = load(path)
    consumers = mirror_usage.setdefault(path, set())
    consumers.add(security_only)
    # A shared archive/security list must keep an archive-capable endpoint.
    security_only = all(consumers)
    def replace(match):
        return match[1] + canonical(match[2], security_only) + match[3]
    rewritten = re.sub(r'(?m)^([ \t]*)([^#\s]+)([^\r\n]*)', replace, text)
    if not re.search(r'(?m)^[ \t]*[^#\s]', text):
        refuse('Empty Ubuntu mirror file')
    planned[path] = rewritten
    return uri


def rewrite_uri(uri, context, suites):
    # Third-party sources are not ours to edit, even when they use Noble suites.
    ubuntu = ('ubuntu-archive-keyring' in context or '/ubuntu' == urlsplit(uri).path.rstrip('/')
              or 'ubuntu.com' in (urlsplit(uri).hostname or ''))
    if not ubuntu:
        return uri
    seen.add(uri)
    if not suites:
        refuse('Ubuntu source has no suites')
    security_only = all(s.endswith('-security') for s in suites)
    if uri.startswith(('mirror+file:', 'file:')):
        return mirror_file(uri, security_only)
    return canonical(uri, security_only)


def legacy(text):
    lines = []
    for line in text.splitlines(keepends=True):
        match = re.match(r'^(\s*deb(?:-src)?\s+(?:\[[^]\r\n]*\]\s+)?)(\S+)(\s+)([^#\r\n]+)', line)
        if match:
            uri = rewrite_uri(match[2], line, [match[4].split()[0]])
            line = line[:match.start(2)] + uri + line[match.end(2):]
        elif line.strip() and not line.lstrip().startswith('#'):
            refuse('Unrecognized legacy source layout')
        lines.append(line)
    return ''.join(lines)


def deb822(text):
    blocks = re.split(r'(\r?\n[ \t]*\r?\n)', text)
    for i in range(0, len(blocks), 2):
        block = blocks[i]
        if re.search(r'(?im)^Enabled:[ \t]*no[ \t]*$', block):
            continue
        suites = re.search(r'(?im)^Suites:[ \t]*([^\r\n]+)', block)
        # Include continuation lines when rewriting the URIs field.
        pattern = r'(?im)^(URIs:[ \t]*)([^\r\n]*(?:\r?\n[ \t]+[^\r\n]*)*)'
        def replace(match):
            values = suites[1].split() if suites else []
            return match[1] + re.sub(r'[^\s]+', lambda m: rewrite_uri(m[0], block, values), match[2])
        blocks[i] = re.sub(pattern, replace, block)
    return ''.join(blocks)


try:
    paths = [root / 'sources.list']
    paths += sorted((root / 'sources.list.d').glob('*.list'))
    paths += sorted((root / 'sources.list.d').glob('*.sources'))
    for path in paths:
        if path.exists():
            original = load(path)
            planned[path] = deb822(original) if path.suffix == '.sources' else legacy(original)
    if not seen:
        refuse('No recognized active Ubuntu sources for fallback')
    # Validate every source and referenced file before making any changes.
    for path, text in planned.items():
        if load(path) != text:
            path.write_bytes(text.encode('utf-8'))
except (ValueError, OSError, UnicodeError) as error:
    sys.exit('Ubuntu mirror fallback refused: ' + str(error))
PY
}

if ! acquire; then
    echo "Primary Ubuntu acquisition failed; trying canonical Ubuntu sources once" >&2
    canonical_sources
    if ! acquire; then
        echo "Canonical Ubuntu acquisition exhausted" >&2
        exit 1
    fi
fi
# Download success is required above. A dpkg failure is final, not a mirror retry.
sudo apt-get "${apt_options[@]}" --no-download install -y --no-install-recommends "${packages[@]}"
