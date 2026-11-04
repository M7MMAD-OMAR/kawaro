//! JPEG metadata segments (S-129): APP1 Exif, APP1 XMP and APP2 ICC_PROFILE.
//!
//! The image crate never writes metadata, so a re-encode strips it. With
//! `keep_metadata` the engine copies these three kinds from the input JPEG
//! into the encoded JPEG, after both files are written: the helpers here
//! extract the segments and inject them back without duplicating any.
//!
//! The parser walks the markers between SOI and the first SOS (or EOI); a
//! malformed length, a missing SOI or a truncated file gives None, never a
//! panic.

/// The APP1 marker, used by Exif and XMP.
pub const APP1: u8 = 0xE1;
/// The APP2 marker, used by the ICC profile chunks.
pub const APP2: u8 = 0xE2;
/// Exif payload prefix.
pub const EXIF: &[u8] = b"Exif\0\0";
/// XMP payload prefix.
pub const XMP: &[u8] = b"http://ns.adobe.com/xap/1.0/\0";
/// ICC profile chunk payload prefix.
pub const ICC_PROFILE: &[u8] = b"ICC_PROFILE\0";

/// One metadata segment: the marker byte and its payload, the two length
/// bytes excluded.
pub type Segment = (u8, Vec<u8>);

/// One parsed marker segment with its place in the file.
struct Raw<'a> {
    marker: u8,
    payload: &'a [u8],
    start: usize,
    end: usize,
}

/// The segments between SOI and the first SOS or EOI, or None when the bytes
/// are not a well formed JPEG.
fn segments(jpeg: &[u8]) -> Option<Vec<Raw<'_>>> {
    if jpeg.len() < 4 || jpeg[0] != 0xFF || jpeg[1] != 0xD8 {
        return None;
    }
    let mut pos = 2;
    let mut out = Vec::new();
    while pos < jpeg.len() {
        if jpeg[pos] != 0xFF {
            return None;
        }
        // Fill bytes may precede a marker.
        let mut marker_at = pos;
        while marker_at + 1 < jpeg.len() && jpeg[marker_at + 1] == 0xFF {
            marker_at += 1;
        }
        if marker_at + 1 >= jpeg.len() {
            return None;
        }
        let marker = jpeg[marker_at + 1];
        pos = marker_at + 2;
        // EOI or the start of scan: the segments end here.
        if marker == 0xD9 || marker == 0xDA {
            return Some(out);
        }
        // Standalone markers carry no length.
        if (0xD0..=0xD7).contains(&marker) || marker == 0x01 {
            continue;
        }
        if pos + 2 > jpeg.len() {
            return None;
        }
        let length = usize::from(u16::from_be_bytes([jpeg[pos], jpeg[pos + 1]]));
        if length < 2 || pos + length > jpeg.len() {
            return None;
        }
        out.push(Raw {
            marker,
            payload: &jpeg[pos + 2..pos + length],
            start: marker_at,
            end: pos + length,
        });
        pos += length;
    }
    None
}

/// An Exif payload is copied only when it carries the TIFF header an APP1
/// Exif segment must have; a damaged payload is skipped, never copied.
fn is_exif(payload: &[u8]) -> bool {
    let Some(rest) = payload.strip_prefix(EXIF) else {
        return false;
    };
    rest.len() >= 4 && (rest.starts_with(b"II*\0") || rest.starts_with(b"MM\0*"))
}

/// True for the three kinds this story copies.
fn is_metadata(raw: &Raw<'_>) -> bool {
    match raw.marker {
        APP1 => is_exif(raw.payload) || raw.payload.starts_with(XMP),
        APP2 => raw.payload.starts_with(ICC_PROFILE),
        _ => false,
    }
}

/// Reads a big or little endian u16 from [at, at + 2); the caller has
/// already bounds checked the slice.
fn read16(bytes: &[u8], at: usize, little: bool) -> u16 {
    let pair = [bytes[at], bytes[at + 1]];
    if little {
        u16::from_le_bytes(pair)
    } else {
        u16::from_be_bytes(pair)
    }
}

/// Sets the Exif Orientation tag (0x0112) in IFD0 to 1. The engine applies
/// the orientation to the pixels before encoding, so a copied tag would
/// rotate the upright output a second time in a viewer. A payload without
/// the tag, or with a TIFF header that cannot be walked, is returned as it is.
fn upright_exif(payload: &[u8]) -> Vec<u8> {
    let mut out = payload.to_vec();
    let Some(tiff) = out.get_mut(EXIF.len()..) else {
        return out;
    };
    if tiff.len() < 8 {
        return out;
    }
    let little = match (tiff[0], tiff[1]) {
        (b'I', b'I') => true,
        (b'M', b'M') => false,
        _ => return out,
    };
    if read16(tiff, 2, little) != 42 {
        return out;
    }
    let at = if little {
        u32::from_le_bytes([tiff[4], tiff[5], tiff[6], tiff[7]])
    } else {
        u32::from_be_bytes([tiff[4], tiff[5], tiff[6], tiff[7]])
    } as usize;
    let Some(count) = tiff.get(at..at + 2).map(|bytes| read16(bytes, 0, little)) else {
        return out;
    };
    for index in 0..usize::from(count) {
        let entry = at + 2 + index * 12;
        let Some(field) = tiff.get(entry..entry + 12) else {
            return out;
        };
        if read16(field, 0, little) != 0x0112 {
            continue;
        }
        // A SHORT of count 1 keeps its value in the first two value bytes.
        let value = if little {
            1u16.to_le_bytes()
        } else {
            1u16.to_be_bytes()
        };
        tiff[entry + 8..entry + 10].copy_from_slice(&value);
        return out;
    }
    out
}

/// The metadata segments of `jpeg`, in file order; an empty list when there
/// are none. None for bytes that are not a well formed JPEG. A copied Exif
/// segment comes back with its orientation tag set to 1.
pub fn extract(jpeg: &[u8]) -> Option<Vec<Segment>> {
    let mut out = Vec::new();
    for raw in segments(jpeg)? {
        if is_metadata(&raw) {
            let payload = if raw.marker == APP1 && is_exif(raw.payload) {
                upright_exif(raw.payload)
            } else {
                raw.payload.to_vec()
            };
            out.push((raw.marker, payload));
        }
    }
    Some(out)
}

/// Puts `extra` after SOI (and after a leading APP0 JFIF segment) in `jpeg`,
/// skipping every segment the file already holds, so nothing is duplicated.
/// None for bytes that are not a well formed JPEG.
pub fn inject(jpeg: &[u8], extra: &[Segment]) -> Option<Vec<u8>> {
    let parsed = segments(jpeg)?;
    // A JFIF header stays first, as the format expects.
    let mut insert_at = 2;
    if let Some(first) = parsed.first()
        && first.start == 2
        && first.marker == 0xE0
        && first.payload.starts_with(b"JFIF\0")
    {
        insert_at = first.end;
    }
    let mut keep: Vec<&Segment> = Vec::new();
    for segment in extra {
        let (marker, payload) = segment;
        let known = parsed
            .iter()
            .any(|raw| raw.marker == *marker && raw.payload == payload.as_slice())
            || keep
                .iter()
                .any(|seen: &&Segment| seen.0 == *marker && &seen.1 == payload);
        if !known {
            keep.push(segment);
        }
    }
    if keep.is_empty() {
        return Some(jpeg.to_vec());
    }
    let mut out = Vec::with_capacity(jpeg.len() + keep.len() * 8);
    out.extend_from_slice(&jpeg[..insert_at]);
    for (marker, payload) in keep {
        let length = payload.len() + 2;
        if length > usize::from(u16::MAX) {
            return None;
        }
        out.push(0xFF);
        out.push(*marker);
        out.extend_from_slice(&(length as u16).to_be_bytes());
        out.extend_from_slice(payload);
    }
    out.extend_from_slice(&jpeg[insert_at..]);
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn segment(marker: u8, payload: &[u8]) -> Vec<u8> {
        let mut out = vec![0xFF, marker];
        out.extend_from_slice(&((payload.len() + 2) as u16).to_be_bytes());
        out.extend_from_slice(payload);
        out
    }

    /// A JPEG made of `segments` between SOI and SOS, with a byte of scan
    /// data and an EOI.
    fn jpeg(segments: &[Vec<u8>]) -> Vec<u8> {
        let mut out = vec![0xFF, 0xD8];
        for segment in segments {
            out.extend_from_slice(segment);
        }
        out.extend_from_slice(&[0xFF, 0xDA, 0x00, 0x02, 0x00, 0x11, 0x22, 0xFF, 0xD9]);
        out
    }

    fn exif() -> Vec<u8> {
        segment(APP1, b"Exif\0\0MM\0\x2a\0\0\0\x08")
    }

    /// A whole Exif segment with one IFD0 entry: Orientation, big endian.
    fn exif_with_orientation(orientation: u16) -> Vec<u8> {
        let mut body = EXIF.to_vec();
        body.extend_from_slice(b"MM\0*\0\0\0\x08");
        body.extend_from_slice(&1u16.to_be_bytes());
        body.extend_from_slice(&0x0112u16.to_be_bytes());
        body.extend_from_slice(&3u16.to_be_bytes());
        body.extend_from_slice(&1u32.to_be_bytes());
        body.extend_from_slice(&orientation.to_be_bytes());
        body.extend_from_slice(&[0, 0]);
        body.extend_from_slice(&0u32.to_be_bytes());
        segment(APP1, &body)
    }

    fn xmp() -> Vec<u8> {
        segment(APP1, b"http://ns.adobe.com/xap/1.0/\0<x:xmpmeta/>")
    }

    fn icc(chunk: u8) -> Vec<u8> {
        let mut payload = ICC_PROFILE.to_vec();
        payload.extend_from_slice(&[1, 2, chunk, 0xAA]);
        segment(APP2, &payload)
    }

    fn jfif() -> Vec<u8> {
        segment(0xE0, b"JFIF\0\x01\x02")
    }

    #[test]
    fn extract_keeps_exif_xmp_and_every_icc_chunk() {
        let file = jpeg(&[
            jfif(),
            exif(),
            xmp(),
            icc(1),
            icc(2),
            segment(0xDB, &[0; 4]),
        ]);
        let found = extract(&file).expect("a well formed JPEG");
        assert_eq!(found.len(), 4, "{found:?}");
        assert_eq!(found[0].0, APP1);
        assert!(found[0].1.starts_with(EXIF));
        assert!(found[1].1.starts_with(XMP));
        assert!(found[2].1.starts_with(ICC_PROFILE));
        assert!(found[3].1.starts_with(ICC_PROFILE));
    }

    #[test]
    fn a_copied_exif_orientation_is_rewritten_to_upright() {
        let file = jpeg(&[jfif(), exif_with_orientation(6)]);
        let found = extract(&file).expect("a well formed JPEG");
        assert_eq!(found.len(), 1);
        let payload = &found[0].1;
        // The value bytes of the one IFD0 entry.
        let at = EXIF.len() + 8 + 2 + 8;
        assert_eq!(&payload[at..at + 2], &[0, 1], "orientation 6 becomes 1");
        assert_eq!(payload.len(), EXIF.len() + 8 + 2 + 12 + 4);
    }

    #[test]
    fn an_exif_without_the_orientation_tag_stays_as_it_is() {
        let mut body = EXIF.to_vec();
        body.extend_from_slice(b"MM\0*\0\0\0\x08");
        body.extend_from_slice(&0u16.to_be_bytes());
        body.extend_from_slice(&0u32.to_be_bytes());
        let file = jpeg(&[jfif(), segment(APP1, &body)]);
        let found = extract(&file).expect("a well formed JPEG");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].1, body);
    }

    #[test]
    fn an_exif_that_cannot_be_walked_is_copied_unchanged() {
        let mut body = EXIF.to_vec();
        // The IFD0 offset points past the end of the payload.
        body.extend_from_slice(b"MM\0*\0\0\x10\x00");
        let file = jpeg(&[jfif(), segment(APP1, &body)]);
        let found = extract(&file).expect("a well formed JPEG");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].1, body);
    }

    #[test]
    fn a_damaged_exif_payload_is_skipped() {
        let damaged = segment(APP1, b"Exif\0\0not a tiff header");
        let file = jpeg(&[jfif(), damaged]);
        assert_eq!(extract(&file), Some(Vec::new()));
    }

    #[test]
    fn no_metadata_is_an_empty_list_not_an_error() {
        let file = jpeg(&[jfif(), segment(0xDB, &[0; 4])]);
        assert_eq!(extract(&file), Some(Vec::new()));
    }

    #[test]
    fn inject_puts_the_segments_after_soi_and_keeps_jfif_first() {
        let plain = jpeg(&[jfif()]);
        let with = inject(&plain, &[(APP1, b"Exif\0\0MM\0*data".to_vec())]).unwrap();
        assert_eq!(&with[..2], &[0xFF, 0xD8]);
        assert_eq!(&with[2..4], &[0xFF, 0xE0], "JFIF stays first");
        let found = extract(&with).unwrap();
        assert_eq!(found.len(), 1);
        assert!(found[0].1.starts_with(EXIF));
        // Everything after the insertion is untouched.
        assert!(with.ends_with(&plain[2 + jfif().len()..]));
    }

    #[test]
    fn inject_does_not_duplicate_a_segment_already_present() {
        let once = jpeg(&[jfif(), exif()]);
        // The same segment the file already holds is not added twice.
        let twice = inject(&once, &extract(&once).unwrap()).unwrap();
        assert_eq!(twice, once, "the copy is not added twice");
        let again = inject(&twice, &extract(&twice).unwrap()).unwrap();
        assert_eq!(again, twice);
    }

    #[test]
    fn inject_of_nothing_leaves_the_file_as_it_is() {
        let plain = jpeg(&[jfif(), icc(1)]);
        assert_eq!(inject(&plain, &[]), Some(plain.clone()));
    }

    #[test]
    fn a_round_trip_keeps_every_copied_segment() {
        let source = jpeg(&[jfif(), exif(), xmp(), icc(1), icc(2)]);
        let metadata = extract(&source).unwrap();
        let target = jpeg(&[jfif()]);
        let out = inject(&target, &metadata).unwrap();
        assert_eq!(extract(&out), Some(metadata));
    }

    #[test]
    fn corrupt_input_gives_none_and_never_panics() {
        assert_eq!(extract(&[]), None);
        assert_eq!(extract(&[0xFF, 0xD8]), None);
        assert_eq!(extract(&[0xFF, 0xD8, 0xFF]), None);
        // Not a JPEG at all.
        assert_eq!(extract(b"not a picture"), None);
        // A segment length that runs past the end.
        assert_eq!(extract(&[0xFF, 0xD8, 0xFF, 0xE1, 0x00, 0x40, 0x00]), None);
        // A length below two is invalid.
        assert_eq!(
            extract(&[0xFF, 0xD8, 0xFF, 0xE1, 0x00, 0x01, 0xFF, 0xD9]),
            None
        );
        // Truncated before SOS or EOI.
        assert_eq!(extract(&[0xFF, 0xD8, 0xFF, 0xE1, 0x00, 0x04, 0x41]), None);
        assert_eq!(inject(&[0xFF, 0xD8], &[(APP1, b"Exif\0\0".to_vec())]), None);
        assert_eq!(inject(b"junk", &[]), None);
    }

    #[test]
    fn inject_handles_many_segments_and_fill_bytes() {
        let plain = vec![
            0xFF, 0xD8, 0xFF, 0xFF, 0xFF, 0xDA, 0x00, 0x02, 0x00, 0xFF, 0xD9,
        ];
        let many: Vec<Segment> = (1..=4)
            .map(|chunk| (APP2, icc(chunk)[4..].to_vec()))
            .collect();
        let out = inject(&plain, &many).unwrap();
        let found = extract(&out).unwrap();
        assert_eq!(found.len(), 4);
    }
}
