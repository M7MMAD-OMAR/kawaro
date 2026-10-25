# ADR 0008: The product is named Kawaro (كوارو)

Status: accepted
Date: 2026-10-26

## Context

The first name was too long to type as a command and to say. The owner asked for a short name that keeps the hive idea and reads the same in Arabic and Latin script, preferably an Arabic root with a Latin ending rather than a foreign word.

## Decision

The product is **Kawaro** in Latin script and **«كوارو»** in Arabic, pronounced ka-wa-ro.

- Source: «كوّارة» (kawwara), an old Arabic word for a beehive. The app is a hive of actions.
- The mark is a traditional domed beehive cut out of one honeycomb cell (`design/brand/build.py`).
- Names checked on 2026-10-26: the crate names `kawaro` and `kawwara` are free on crates.io, and `kawaro.com`, `.app`, `.dev` and `.io` did not resolve.
- Rejected: foreign-word candidates, because a dropped silent letter can change the vowel for the reader, and other Arabic roots (cell, bees, honey) with a weaker link to the hive.

What changes: the binaries (`kawaro`, `kawarod`), crate and package names, the app ID `io.github.m7mmad_omar.Kawaro`, config and runtime paths, `KAWARO_*` environment variables, the GitHub repository `M7MMAD-OMAR/kawaro`, the website `https://kawaro.sbarah.com`, every document, and the brand assets.

## Consequences

- Git history, tags and old release assets keep the earlier name; history is not rewritten.
- This is a pre-release beta, so existing installs get no config or service migration; a reinstall starts clean.
- The in-app update feed and its signing key are unchanged in format. The feed URL follows the new repository name.
- The old website host is removed from the Cloudflare route; no redirect is kept.
