//! Raster header checks shared by every supported package layout.

use std::collections::BTreeMap;

use super::{Provenance, Violation, error};

const MIN_IMAGE_DIMENSION_PX: u32 = 5;

pub(super) fn check_entries(entries: &BTreeMap<String, Vec<u8>>, violations: &mut Vec<Violation>) {
    for (path, data) in entries.iter().filter(|(path, _)| is_raster(path)) {
        let Some((width, height)) = raster_dimensions(data) else {
            violations.push(error(
                "unreadable-raster",
                path,
                "could not read image dimensions from header",
                Provenance::FormatRequirement,
            ));
            continue;
        };
        if width.min(height) <= MIN_IMAGE_DIMENSION_PX {
            violations.push(Violation::advisory(
                "invisible-raster",
                path,
                format!(
                    "image dimensions {width}x{height} are at or below {MIN_IMAGE_DIMENSION_PX}px"
                ),
            ));
        }
    }
}

fn is_raster(path: &str) -> bool {
    let path = path.to_ascii_lowercase();
    path.ends_with(".png")
        || path.ends_with(".gif")
        || path.ends_with(".jpg")
        || path.ends_with(".jpeg")
}

fn raster_dimensions(data: &[u8]) -> Option<(u32, u32)> {
    if data.starts_with(b"\x89PNG\r\n\x1a\n") && data.len() >= 24 {
        return Some((
            u32::from_be_bytes(data[16..20].try_into().ok()?),
            u32::from_be_bytes(data[20..24].try_into().ok()?),
        ));
    }
    if (data.starts_with(b"GIF87a") || data.starts_with(b"GIF89a")) && data.len() >= 10 {
        return Some((
            u16::from_le_bytes(data[6..8].try_into().ok()?).into(),
            u16::from_le_bytes(data[8..10].try_into().ok()?).into(),
        ));
    }
    jpeg_dimensions(data)
}

fn jpeg_dimensions(data: &[u8]) -> Option<(u32, u32)> {
    if !data.starts_with(&[0xff, 0xd8]) {
        return None;
    }
    let mut offset = 2;
    while offset < data.len() {
        if data[offset] != 0xff {
            return None;
        }
        while data.get(offset) == Some(&0xff) {
            offset += 1;
        }
        let marker = *data.get(offset)?;
        offset += 1;
        if marker == 0x01 || (0xd0..=0xd7).contains(&marker) {
            continue;
        }
        if marker == 0xd9 {
            return None;
        }
        let segment_length =
            u16::from_be_bytes(data.get(offset..offset + 2)?.try_into().ok()?) as usize;
        if segment_length < 2 || offset.checked_add(segment_length)? > data.len() {
            return None;
        }
        if matches!(marker, 0xc0..=0xc3 | 0xc5..=0xc7 | 0xc9..=0xcb | 0xcd..=0xcf) {
            if segment_length < 7 {
                return None;
            }
            let height = u16::from_be_bytes(data.get(offset + 3..offset + 5)?.try_into().ok()?);
            let width = u16::from_be_bytes(data.get(offset + 5..offset + 7)?.try_into().ok()?);
            return Some((width.into(), height.into()));
        }
        offset += segment_length;
    }
    None
}
