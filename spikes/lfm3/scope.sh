#!/usr/bin/env bash
# Run any spike command inside the memory bounded scope (see CLAUDE.md, heavy work).
# CPUQuota is 400% instead of 200% so the speed numbers resemble a 4 core laptop.
exec systemd-run --user --scope -q -p MemoryMax=6G -p CPUQuota=400% "$@"
