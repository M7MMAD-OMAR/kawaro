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

    /// A private directory for one test, so parallel tests never share files.
    fn scratch(test: &str) -> std::path::PathBuf {
        let dir = Path::new(env!("OUT_DIR")).join(format!("s018-mutants-{test}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Encodes an 8 bit `w` x `h` PNG of `color` (no tRNS chunk) into bytes.
    fn encode(w: u32, h: u32, color: png::ColorType, data: &[u8]) -> Vec<u8> {
        let mut bytes = Vec::new();
        let mut e = png::Encoder::new(&mut bytes, w, h);
        e.set_color(color);
        e.set_depth(png::BitDepth::Eight);
        let mut wr = e.write_header().unwrap();
        wr.write_image_data(data).unwrap();
        drop(wr);
        bytes
    }

    fn crc32(data: &[u8]) -> u32 {
        let mut crc = 0xFFFF_FFFFu32;
        for &b in data {
            crc ^= u32::from(b);
            for _ in 0..8 {
                crc = if crc & 1 != 0 {
                    (crc >> 1) ^ 0xEDB8_8320
                } else {
                    crc >> 1
                };
            }
        }
        !crc
    }

    fn rgba(buf: &SharedPixelBuffer<Rgba8Pixel>, x: u32, y: u32) -> [u8; 4] {
        let p = buf.as_slice()[(y * buf.width() + x) as usize];
        [p.r, p.g, p.b, p.a]
    }

    #[test]
    fn ac24_fit_rounds_half_up_to_the_nearest_pixel() {
        // 59 * 52 / 100 = 30.68: rounded, not truncated.
        assert_eq!(fit(100, 59, 52), (52, 31));
        assert_eq!(fit(59, 100, 52), (31, 52));
    }

    #[test]
    fn ac24_interlaced_png_gives_none() {
        let dir = scratch("interlaced");
        let plain = encode(1, 1, png::ColorType::Rgba, &[10, 20, 30, 255]);
        // IHDR data runs from byte 16 to 29; its interlace method is byte 28
        // and its CRC (over type and data, bytes 12 to 29) follows at 29.
        assert_eq!(&plain[12..16], b"IHDR");
        assert_eq!(plain[28], 0);
        let mut laced = plain.clone();
        laced[28] = 1;
        let crc = crc32(&laced[12..29]);
        laced[29..33].copy_from_slice(&crc.to_be_bytes());
        // For one pixel the Adam7 pass 1 data is the plain data, so the file
        // is valid and differs only in the interlace flag.
        let (p, i) = (dir.join("plain.png"), dir.join("interlaced.png"));
        std::fs::write(&p, &plain).unwrap();
        std::fs::write(&i, &laced).unwrap();
        let reader = png::Decoder::new(std::fs::File::open(&i).unwrap())
            .read_info()
            .unwrap();
        assert!(reader.info().interlaced, "the fixture is interlaced");
        assert!(decode_fit(&p, 52).is_some(), "the plain twin decodes");
        assert!(decode_fit(&i, 52).is_none(), "interlaced is refused");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn ac24_zero_target_size_gives_none() {
        let dir = scratch("zero-max");
        let p = dir.join("one.png");
        std::fs::write(&p, encode(2, 2, png::ColorType::Rgba, &[200; 16])).unwrap();
        assert!(decode_fit(&p, 2).is_some(), "the file itself decodes");
        assert!(decode_fit(&p, 0).is_none(), "a zero size gives no buffer");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn ac24_grey_without_alpha_becomes_opaque_grey() {
        let dir = scratch("grey");
        let p = dir.join("grey.png");
        // 4 x 2, left half 200, right half 60.
        let row = [200, 200, 60, 60];
        std::fs::write(
            &p,
            encode(4, 2, png::ColorType::Grayscale, &[row, row].concat()),
        )
        .unwrap();
        let buf = decode_fit(&p, 2).unwrap();
        assert_eq!((buf.width(), buf.height()), (2, 1));
        assert_eq!(rgba(&buf, 0, 0), [200, 200, 200, 255]);
        assert_eq!(rgba(&buf, 1, 0), [60, 60, 60, 255]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn ac24_rgb_without_alpha_keeps_each_channel_opaque() {
        let dir = scratch("rgb");
        let p = dir.join("rgb.png");
        // 4 x 2, left half (210, 120, 30), right half (15, 90, 240).
        let row = [210, 120, 30, 210, 120, 30, 15, 90, 240, 15, 90, 240];
        std::fs::write(&p, encode(4, 2, png::ColorType::Rgb, &[row, row].concat())).unwrap();
        let buf = decode_fit(&p, 2).unwrap();
        assert_eq!((buf.width(), buf.height()), (2, 1));
        assert_eq!(rgba(&buf, 0, 0), [210, 120, 30, 255]);
        assert_eq!(rgba(&buf, 1, 0), [15, 90, 240, 255]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn ac24_last_output_row_is_written_in_place() {
        let dir = scratch("rows");
        let p = dir.join("rows.png");
        // 4 x 4 RGBA: top half A, bottom half B, so a 2 x 2 output has A on
        // row 0 and B on row 1.
        let a = [250, 10, 10, 255];
        let b = [10, 10, 250, 255];
        let mut data = Vec::new();
        for y in 0..4 {
            for _ in 0..4 {
                data.extend_from_slice(if y < 2 { &a } else { &b });
            }
        }
        std::fs::write(&p, encode(4, 4, png::ColorType::Rgba, &data)).unwrap();
        let buf = decode_fit(&p, 2).unwrap();
        assert_eq!((buf.width(), buf.height()), (2, 2));
        for x in 0..2 {
            assert_eq!(rgba(&buf, x, 0), a, "row 0, column {x}");
            assert_eq!(rgba(&buf, x, 1), b, "row 1, column {x}");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
