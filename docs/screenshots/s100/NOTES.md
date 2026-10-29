# S-100 evidence (Windows 11 VM, 2026-10-29)

- `vm-before-fix-guid-only.txt`: new ComponentGuids only. Every upgrade rc=1603, folder left with `actions` only.
- `msi-log-before-fix-excerpt.txt`: `Disallowing installation of component ... higher versioned keyfile`, `Action: Null`, error 2753.
- `dblclick-fails-2753-first-attempt.png`: the double click upgrade showing error 2753.
- `vm-results-fixed.txt`, `msi-log-after-fix-excerpt.txt`: with File.Version on the exes, rc=0 for all four scenarios, new exe hashes, config kept, uninstall clean.
- Package hashes: 0.4.3 kawaro.exe 84b36a90..., kawarod.exe 8654078b...; 0.4.4 kawaro.exe 8897127c..., kawarod.exe 4b9612d9...
