//! The centre cell picture (S-018h): a cached thumbnail decoded one row at a
//! time and box-filtered straight into a buffer no larger than the square
//! inscribed in the centre hexagon. The full-size image is never held, so a
//! 1024 px thumbnail costs a few kilobytes, not 4 MB (AC24, AC27).

use slint::{Rgba8Pixel, SharedPixelBuffer};
use std::path::Path;

/// `(w, h)` scaled down so the longer side is at most `max`, aspect kept.
/// Never scaled up.
pub(crate) fn fit(w: u32, h: u32, max: u32) -> (u32, u32) {
    let long = w.max(h);
    if long <= max {
        return (w, h);
    }
    let scale = |v: u32| {
        ((u64::from(v) * u64::from(max) + u64::from(long) / 2) / u64::from(long)).max(1) as u32
    };
    (scale(w), scale(h))
}

/// Sums of one output row: premultiplied red, green, blue, then alpha and
/// the number of source pixels.
struct Row {
    acc: Vec<[u64; 5]>,
}

impl Row {
    fn add(&mut self, dx: usize, [r, g, b, a]: [u8; 4]) {
        let a64 = u64::from(a);
        let s = &mut self.acc[dx];
        s[0] += u64::from(r) * a64;
        s[1] += u64::from(g) * a64;
        s[2] += u64::from(b) * a64;
        s[3] += a64;
        s[4] += 1;
    }

    /// Writes the averages into `out` and starts the next row.
    fn flush(&mut self, out: &mut [Rgba8Pixel]) {
        for (s, p) in self.acc.iter_mut().zip(out) {
            if let Some(alpha) = s[3].checked_div(s[4]) {
                let c = |v: u64| v.checked_div(s[3]).unwrap_or(0) as u8;
                *p = Rgba8Pixel::new(c(s[0]), c(s[1]), c(s[2]), alpha as u8);
            }
            *s = [0; 5];
        }
    }
}

/// Decodes the PNG at `path` into at most `max` pixels on its longer side.
/// `None` for anything it cannot read, and for interlaced files, which
/// cannot be read one row at a time.
pub(crate) fn decode_fit(path: &Path, max: u32) -> Option<SharedPixelBuffer<Rgba8Pixel>> {
    let file = std::fs::File::open(path).ok()?;
    let mut decoder = png::Decoder::new(std::io::BufReader::new(file));
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().ok()?;
    let (w, h, interlaced) = {
        let i = reader.info();
        (i.width, i.height, i.interlaced)
    };
    if interlaced || w == 0 || h == 0 || max == 0 {
        return None;
    }
    let channels = match reader.output_color_type().0 {
        png::ColorType::Grayscale => 1,
        png::ColorType::GrayscaleAlpha => 2,
        png::ColorType::Rgb => 3,
        png::ColorType::Rgba => 4,
        png::ColorType::Indexed => return None,
    };
    let (tw, th) = fit(w, h, max);
    let mut out = SharedPixelBuffer::<Rgba8Pixel>::new(tw, th);
    let mut row = Row {
        acc: vec![[0; 5]; tw as usize],
    };
    let dest = |v: u32, of: u32, to: u32| (u64::from(v) * u64::from(to) / u64::from(of)) as usize;
    let mut current = 0;
    for y in 0..h {
        let src = reader.next_row().ok()??;
        let data = src.data();
        let dy = dest(y, h, th);
        if dy != current {
            let start = current * tw as usize;
            row.flush(&mut out.make_mut_slice()[start..start + tw as usize]);
            current = dy;
        }
        for (x, px) in data.chunks_exact(channels).take(w as usize).enumerate() {
            let rgba = match *px {
                [l] => [l, l, l, 255],
                [l, a] => [l, l, l, a],
                [r, g, b] => [r, g, b, 255],
                [r, g, b, a] => [r, g, b, a],
                _ => return None,
            };
            row.add(dest(x as u32, w, tw), rgba);
        }
    }
    let start = current * tw as usize;
    row.flush(&mut out.make_mut_slice()[start..start + tw as usize]);
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fit_keeps_the_aspect_and_never_grows() {
        assert_eq!(fit(1024, 512, 52), (52, 26));
        assert_eq!(fit(512, 1024, 52), (26, 52));
        assert_eq!(fit(40, 20, 52), (40, 20));
        assert_eq!(fit(1000, 1, 52), (52, 1));
    }

    #[test]
    fn grey_and_alpha_rows_average_into_the_small_buffer() {
        let dir = Path::new(env!("OUT_DIR")).join("s018h-unit-preview");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("grey.png");
        let w = std::io::BufWriter::new(std::fs::File::create(&path).unwrap());
        let mut e = png::Encoder::new(w, 4, 2);
        e.set_color(png::ColorType::GrayscaleAlpha);
        // Left half white and opaque, right half transparent.
        let row = [255, 255, 255, 255, 0, 0, 0, 0];
        let mut wr = e.write_header().unwrap();
        wr.write_image_data(&[row, row].concat()).unwrap();
        drop(wr);
        let buf = decode_fit(&path, 2).unwrap();
        assert_eq!((buf.width(), buf.height()), (2, 1));
        let px = buf.as_slice();
        assert_eq!(px[0], Rgba8Pixel::new(255, 255, 255, 255));
        assert_eq!(px[1].a, 0);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
