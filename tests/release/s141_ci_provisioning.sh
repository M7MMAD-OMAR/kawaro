#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# S141: mock-only Ubuntu 24.04 MinGW provisioning. No Cargo or host APT.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
export ROOT
python3 - <<'PY'
import json
import os
from pathlib import Path
import shlex
import subprocess
import tempfile
import unittest
import yaml

ROOT = Path(os.environ['ROOT'])
HELPER = ROOT / 'scripts/ci/install-mingw.sh'
AZURE = 'http://azure.archive.ubuntu.com/ubuntu'
ARCHIVE = 'https://archive.ubuntu.com/ubuntu'
SECURITY = 'https://security.ubuntu.com/ubuntu'
PACKAGES = {'gcc-mingw-w64-x86-64', 'g++-mingw-w64-x86-64'}
THIRD = '# third party stays bytewise\ndeb [arch=amd64 signed-by=/usr/share/keyrings/ms.gpg] https://packages.microsoft.com/ubuntu/24.04/prod noble main\n'
LEGACY = ('# preserve this comment: http://azure.archive.ubuntu.com/ubuntu\n'
          'deb [arch=amd64 signed-by=/usr/share/keyrings/ubuntu-archive-keyring.gpg] '
          + AZURE + ' noble main restricted universe multiverse\n'
          'deb ' + AZURE + ' noble-updates main universe\n')
DEB822 = ('# Ubuntu archive\nTypes: deb deb-src\nURIs: ' + AZURE + '\n'
          'Suites: noble noble-updates noble-backports\nComponents: main universe restricted multiverse\n'
          'Architectures: amd64\nSigned-By: /usr/share/keyrings/ubuntu-archive-keyring.gpg\n\n'
          '# Separate security\nTypes: deb\nURIs: http://security.ubuntu.com/ubuntu\n'
          'Suites: noble-security\nComponents: main universe\nArchitectures: amd64\n'
          'Signed-By: /usr/share/keyrings/ubuntu-archive-keyring.gpg\n')

# These stubs never invoke real sudo, apt-get or timeout. sudo delegates only
# to the two stubs, never to arbitrary commands. timeout simulates 124, not time.
MOCK = r'''#!/usr/bin/env python3
import json, os, pathlib, subprocess, sys
name = pathlib.Path(sys.argv[0]).name
args = sys.argv[1:]
root = pathlib.Path(os.environ['APT_CONFIG_ROOT'])
state = pathlib.Path(os.environ['MOCK_STATE'])
log = pathlib.Path(os.environ['MOCK_LOG'])
def record(**extra):
    with log.open('a') as f:
        f.write(json.dumps(dict(cmd=name, args=args, **extra)) + '\n')
def delegate(argv, bounded=False):
    if not argv or pathlib.Path(argv[0]).name not in ('apt-get', 'timeout'):
        record(refused=True)
        sys.exit(97)
    env = dict(os.environ)
    if bounded:
        env['MOCK_BOUNDED'] = '1'
    sys.exit(subprocess.call([str(pathlib.Path(__file__).parent / pathlib.Path(argv[0]).name), *argv[1:]], env=env))
if name == 'sudo':
    record()
    delegate(args)
if name == 'timeout':
    record()
    # Accept standard timeout options, then a seconds duration and command.
    i = 0
    while i < len(args) and args[i].startswith('-'):
        if args[i] in ('-k', '--kill-after', '-s', '--signal'):
            i += 2
        else:
            i += 1
    if i >= len(args):
        sys.exit(97)
    delegate(args[i+1:], True)
if name != 'apt-get':
    sys.exit(97)
phase = 'update' if 'update' in args else ('download' if '--download-only' in args or '-d' in args else 'install')
counts = json.loads(state.read_text()) if state.exists() else {}
counts[phase] = counts.get(phase, 0) + 1
state.write_text(json.dumps(counts))
record(phase=phase, bounded=os.environ.get('MOCK_BOUNDED') == '1',
       sources={str(p.relative_to(root)): p.read_text() for p in root.rglob('*') if p.is_file()})
mode = os.environ['MOCK_MODE']
if mode == 'install-fail' and phase == 'install':
    sys.exit(42)
if mode in ('update-exhaust', 'download-exhaust') and phase == mode.split('-')[0]:
    sys.exit(100)
if mode.startswith(phase + '-') and counts[phase] == 1:
    sys.exit(124 if mode.endswith('timeout') else 100)
sys.exit(0)
'''

class Provisioning(unittest.TestCase):
    def setUp(self):
        compile(MOCK, '<safe APT mocks>', 'exec')
        # Keep even the default temporary tree out of the small /tmp tmpfs.
        self.tmp = tempfile.TemporaryDirectory(prefix='s141-apt-', dir=Path.home())
        self.addCleanup(self.tmp.cleanup)
        self.base = Path(self.tmp.name)
        self.apt = self.base / 'apt'
        (self.apt / 'sources.list.d').mkdir(parents=True)
        self.put('sources.list', LEGACY)
        self.put('sources.list.d/microsoft.list', THIRD)
        self.bin = self.base / 'bin'
        self.bin.mkdir()
        for name in ('sudo', 'apt-get', 'timeout'):
            p = self.bin / name
            p.write_text(MOCK)
            p.chmod(0o755)
        self.log = self.base / 'commands.jsonl'
        self.env = dict(os.environ, APT_CONFIG_ROOT=str(self.apt),
                        PATH=str(self.bin) + os.pathsep + os.environ['PATH'],
                        TMPDIR=str(self.base), MOCK_STATE=str(self.base / 'state.json'),
                        MOCK_LOG=str(self.log), CI='true')

    def put(self, path, text):
        (self.apt / path).write_text(text)

    def snapshot(self):
        return {str(p.relative_to(self.apt)): p.read_text() for p in self.apt.rglob('*') if p.is_file()}

    def run_helper(self, mode='success'):
        self.assertTrue(HELPER.is_file(), 'missing planned scripts/ci/install-mingw.sh')
        self.env['MOCK_MODE'] = mode
        result = subprocess.run(['bash', str(HELPER)], env=self.env, cwd=ROOT,
                                text=True, capture_output=True, timeout=15)
        events = [json.loads(line) for line in self.log.read_text().splitlines()] if self.log.exists() else []
        self.assertFalse(any(e.get('refused') for e in events), 'sudo attempted unsafe delegation')
        calls = [e for e in events if e['cmd'] == 'apt-get']
        self.assert_apt_contract(calls, events)
        return result, calls

    def assert_apt_contract(self, calls, events):
        for e in calls:
            args = e['args']
            joined = ' '.join(args)
            for option in ('Acquire::Retries=2', 'Acquire::http::Timeout=30', 'Acquire::https::Timeout=30'):
                self.assertIn(option, joined)
            self.assertNotRegex(joined.lower(), r'allow-unauthenticated|allow-insecure|allow-downgrade-to-insecure|trusted[=:]yes|verify-peer=false|check-valid-until=false')
            if e['phase'] == 'update':
                self.assertIn('APT::Update::Error-Mode=any', joined)
            else:
                self.assertIn('--no-install-recommends', args)
                self.assertIn('install', args)
                self.assertIn('-y', args)
                requested = {a for a in args if 'mingw' in a}
                self.assertEqual(requested, PACKAGES, 'retain both x64 thread variants via GCC/G++ meta packages')
            if e['phase'] == 'install':
                self.assertIn('--no-download', args)
                self.assertFalse(e['bounded'], 'never kill/retry a dpkg install')
            else:
                self.assertTrue(e['bounded'], 'network command needs wall-clock timeout')
        timers = [e['args'] for e in events if e['cmd'] == 'timeout']
        for args in timers:
            joined = ' '.join(args)
            self.assertRegex(joined, r'(?:--kill-after(?:=| )|-k ?)(?:10s|10)(?: |$)')
            expected = '90' if 'update' in args else '240'
            self.assertTrue(any(a in (expected, expected+'s') for a in args), joined)
            self.assertTrue('update' in args or '--download-only' in args or '-d' in args, 'timeout must never wrap installation/helper')
        phases = [e['phase'] for e in calls]
        self.assertLessEqual(phases.count('install'), 1)
        if 'install' in phases:
            self.assertEqual(phases[-2:], ['download', 'install'])
        self.assertLessEqual(phases.count('update'), 2)
        self.assertLessEqual(phases.count('download'), 2)

    def expect(self, mode, phases, success=True):
        result, calls = self.run_helper(mode)
        self.assertEqual(result.returncode == 0, success, result.stdout + result.stderr)
        self.assertEqual([e['phase'] for e in calls], phases)
        return calls

    def fallback(self, mode):
        before = self.snapshot()
        phases = ['update'] if mode.startswith('update-') else ['update', 'download']
        calls = self.expect(mode, phases + ['update', 'download', 'install'])
        self.assertEqual(calls[0]['sources'], before, 'primary must use original source bytes')
        self.assertEqual(calls[len(phases)]['sources'], self.snapshot(), 'rewrite must precede fallback update')
        self.assertNotEqual(before, self.snapshot())
        return before

    def test_01_helper_exists(self):
        self.assertTrue(HELPER.is_file(), 'missing planned provisioning helper')

    def test_02_workflow_only_provision_step_changes(self):
        workflow = '.github/workflows/ci.yml'
        old = yaml.safe_load((ROOT / 'tests/fixtures/ci/s141-before-provisioning.yml').read_text())
        new = yaml.safe_load((ROOT / workflow).read_text())
        cross = new['jobs']['windows-cross']
        self.assertEqual(cross['runs-on'], 'ubuntu-24.04')
        steps = cross['steps']
        matches = [i for i, s in enumerate(steps) if 'scripts/ci/install-mingw.sh' in s.get('run', '')]
        self.assertEqual(matches, [1], 'replace broad APT with helper, immediately after checkout')
        step = steps[1]
        self.assertEqual(step.get('timeout-minutes'), 15)
        self.assertIn(shlex.split(step['run']), [['scripts/ci/install-mingw.sh'], ['bash', 'scripts/ci/install-mingw.sh']])
        self.assertNotIn('env', step, 'production must not inject fixture root or command overrides')
        new['jobs']['windows-cross']['steps'][1] = old['jobs']['windows-cross']['steps'][1]
        self.assertEqual(new, old, 'all downstream Windows check/build/DLL/S085 and other jobs stay unchanged')

    def test_03_no_auth_or_compiler_substitution(self):
        self.assertTrue(HELPER.is_file(), 'helper absent')
        text = HELPER.read_text().lower()
        self.assertNotRegex(text, r'--allow-unauthenticated|trusted=yes|allowinsecurerepositories|verify-peer[=:]false|check-valid-until[=:]false|update-alternatives')
        self.assertNotRegex(text, r'(?m)^\s*(?:export\s+)?(?:cc|cxx|rustflags|cargo_target_.*_linker)=')
        self.assertIn('/etc/apt', text, 'production APT root must remain /etc/apt')

    def test_04_primary_success_no_rewrite(self):
        before = self.snapshot()
        self.expect('success', ['update', 'download', 'install'])
        self.assertEqual(self.snapshot(), before)

    def test_05_update_failure_falls_back_once(self):
        self.fallback('update-fail')

    def test_06_download_failure_falls_back_once(self):
        self.fallback('download-fail')

    def test_07_update_timeout_falls_back_once(self):
        self.fallback('update-timeout')

    def test_08_download_timeout_falls_back_once(self):
        self.fallback('download-timeout')

    def test_09_update_exhaustion_no_install(self):
        self.expect('update-exhaust', ['update', 'update'], False)

    def test_10_download_exhaustion_no_install(self):
        self.expect('download-exhaust', ['update', 'download', 'update', 'download'], False)

    def test_11_install_failure_never_retried(self):
        before = self.snapshot()
        self.expect('install-fail', ['update', 'download', 'install'], False)
        self.assertEqual(self.snapshot(), before)

    def test_12_legacy_preserves_every_other_byte(self):
        self.fallback('update-fail')
        expected = LEGACY.replace('] ' + AZURE, '] ' + ARCHIVE).replace('deb ' + AZURE, 'deb ' + ARCHIVE)
        self.assertEqual((self.apt / 'sources.list').read_text(), expected)
        self.assertEqual((self.apt / 'sources.list.d/microsoft.list').read_text(), THIRD)

    def test_13_deb822_separate_security_preserved(self):
        self.put('sources.list', '# no active legacy sources\n')
        self.put('sources.list.d/ubuntu.sources', DEB822)
        self.fallback('download-fail')
        self.assertEqual((self.apt / 'sources.list.d/ubuntu.sources').read_text(),
                         DEB822.replace('URIs: ' + AZURE, 'URIs: ' + ARCHIVE).replace('URIs: http://security.ubuntu.com/ubuntu', 'URIs: ' + SECURITY))
        self.assertEqual((self.apt / 'sources.list.d/microsoft.list').read_text(), THIRD)

    def test_14_shared_file_mirror_keeps_all_pockets(self):
        sources = DEB822.split('\n\n')[0].replace(AZURE, 'mirror+file:' + str(self.apt / 'apt-mirrors.txt')).replace('noble-backports', 'noble-backports noble-security') + '\n'
        self.put('sources.list', '# legacy disabled\n')
        self.put('sources.list.d/ubuntu.sources', sources)
        mirror = '# preserve mirror comment\n' + AZURE + '\n'
        self.put('apt-mirrors.txt', mirror)
        self.fallback('update-fail')
        self.assertEqual((self.apt / 'sources.list.d/ubuntu.sources').read_text(), sources)
        self.assertEqual((self.apt / 'apt-mirrors.txt').read_text(), mirror.replace(AZURE, ARCHIVE))
        self.assertEqual((self.apt / 'sources.list.d/microsoft.list').read_text(), THIRD)

    def test_15_already_canonical_idempotent_fallback(self):
        self.put('sources.list', LEGACY.replace('] ' + AZURE, '] ' + ARCHIVE).replace('deb ' + AZURE, 'deb ' + ARCHIVE))
        before = self.snapshot()
        self.expect('update-fail', ['update', 'update', 'download', 'install'])
        self.assertEqual(self.snapshot(), before)
        (self.base / 'state.json').unlink()
        self.log.unlink()
        self.expect('download-fail', ['update', 'download', 'update', 'download', 'install'])
        self.assertEqual(self.snapshot(), before)

    def test_16_unknown_ubuntu_reference_fails_closed(self):
        self.put('sources.list', LEGACY.replace(AZURE, 'https://unknown.invalid/ubuntu'))
        self.put('sources.list.d/ubuntu.sources', DEB822)
        before = self.snapshot()
        result, calls = self.run_helper('update-fail')
        self.assertNotEqual(result.returncode, 0)
        self.assertLessEqual(len(calls), 1, 'unknown mirror must not be retried or installed')
        self.assertTrue(all(e['phase'] == 'update' for e in calls))
        self.assertEqual(self.snapshot(), before, 'unknown layout must not partially rewrite')
        self.assertTrue(result.stdout.strip() or result.stderr.strip(), 'refusal must be diagnostic')

    def shared_separate_stanzas(self, security_first):
        mirror_path = self.apt / 'apt-mirrors.txt'
        archive, security = DEB822.split('\n\n')
        archive = archive.replace(AZURE, 'mirror+file:' + str(mirror_path))
        security = security.replace('http://security.ubuntu.com/ubuntu', 'mirror+file:' + str(mirror_path))
        blocks = [security.rstrip('\n'), archive] if security_first else [archive, security.rstrip('\n')]
        sources = '\n\n'.join(blocks) + '\n'
        self.put('sources.list', '# legacy disabled\n')
        self.put('sources.list.d/ubuntu.sources', sources)
        # The URL in the comment is not a URI entry and must stay untouched.
        mirror = '# shared archive and security: ' + AZURE + '\n  ' + AZURE + '\t# priority stays\n'
        self.put('apt-mirrors.txt', mirror)
        expected = {str(p.relative_to(self.apt)): p.read_bytes()
                    for p in self.apt.rglob('*') if p.is_file()}
        expected['apt-mirrors.txt'] = mirror.replace('\n  ' + AZURE, '\n  ' + ARCHIVE).encode()
        self.fallback('update-fail')
        actual = {str(p.relative_to(self.apt)): p.read_bytes()
                  for p in self.apt.rglob('*') if p.is_file()}
        self.assertEqual(actual, expected, 'only the mirror URI entry changes, both stanzas and all metadata stay bytewise')

    def test_17_shared_separate_stanzas_archive_first(self):
        self.shared_separate_stanzas(security_first=False)

    def test_18_shared_separate_stanzas_security_first(self):
        self.shared_separate_stanzas(security_first=True)

    def test_19_workflow_check_without_git(self):
        git = self.bin / 'git'
        marker = self.base / 'git-called'
        git.write_text('#!/bin/sh\nprintf called > ' + shlex.quote(str(marker)) + '\nexit 128\n')
        git.chmod(0o755)
        previous_path = os.environ['PATH']
        try:
            os.environ['PATH'] = str(self.bin) + os.pathsep + previous_path
            self.test_02_workflow_only_provision_step_changes()
            self.assertFalse(marker.exists(), 'workflow assertions must not require VCS after checkout')
        finally:
            os.environ['PATH'] = previous_path

unittest.main(verbosity=2)
PY
