# Welcome demonstration frames

Every file in this directory is generated, never hand drawn.

- `frame-{lp,lc,dp,dc}-NN.png` are the demonstration frames of the
  first-run welcome window (S-087), rendered by
  `scripts/gen-welcome-frames.sh` from `crates/ui/ui/welcome-frames.slint`.
  The scene draws the hive with `crates/ui/components/hex-cell.slint`, the
  same hex cell component the real hive uses, and takes every colour,
  spacing and size from `design/tokens.json`. `lp` and `dp` show the hive at
  the pointer, `lc` and `dc` centred; `l` and `d` are the light and dark
  schemes. Sixteen frames per sequence.

No frame asset may be committed without that generator. The frames and the
sample image of the welcome window are embedded into the shipped binary
together, under the 2 MB budget of S-087 AC2.

Licence: generated artwork of this project, from this project's own
components and design tokens, distributed under the repository licence
(GPLv3, see the top level LICENSE), like every other asset here.
