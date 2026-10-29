# Welcome resources

This directory holds no frame images. Since S-094c the Introduction film of
the welcome window is a live vector scene (`crates/ui/ui/film.slint`) driven
by `kawaro_ui::film`, drawn at the displayed size and scale, so it is sharp
on every screen. The old raster frames (576x324, upscaled) and their
generator were removed.

Any raster frame that is ever added here must be at least 1680x945, which is
three times `film-max-width` (`tests/s094c_film.rs`).

The bundled sample image of the welcome window lives in
`tests/fixtures/images/welcome-sample.png`. Licence: GPLv3, like every other
asset of this project.
