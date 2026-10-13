//! Inspect original central records before zip-rs can deduplicate or allocate them.

use std::collections::{BTreeMap, BTreeSet};

use crate::types::Violation;

use super::{
    MAX_ENTRY_COUNT, MAX_PACKAGE_BYTES, ZIP_INPUT, bound_error, duplicate_error, input_error,
    validate_entry_name,
};

pub(super) struct Directory {
    pub count: usize,
    pub start: usize,
}

pub(super) fn inspect(bytes: &[u8]) -> Result<Directory, Violation> {
    // EOCD has a 22-byte fixed header and at most a u16-sized comment. Only
    // accept a marker whose declared comment reaches the end of the input.
    let search_start = bytes.len().saturating_sub(22 + u16::MAX as usize);
    let end = (search_start..bytes.len().saturating_sub(21))
        .rev()
        .find(|&offset| {
            bytes.get(offset..offset + 4) == Some(b"PK\x05\x06")
                && read_u16(bytes, offset + 20)
                    .is_ok_and(|comment| offset + 22 + comment as usize == bytes.len())
        })
        .ok_or_else(|| malformed("missing or truncated ZIP end record"))?;

    if read_u16(bytes, end + 4)? != 0 || read_u16(bytes, end + 6)? != 0 {
        return Err(malformed("multi-disk ZIP packages are unsupported"));
    }
    let count = read_u16(bytes, end + 10)?;
    if read_u16(bytes, end + 8)? != count {
        return Err(malformed("ZIP entry counts disagree"));
    }
    let size = read_u32(bytes, end + 12)?;
    let offset = read_u32(bytes, end + 16)?;
    let locator = end.checked_sub(20);
    let zip64 =
        locator.filter(|&position| bytes.get(position..position + 4) == Some(b"PK\x06\x07"));
    let (count, size, offset, directory_end) = if let Some(locator) = zip64 {
        zip64_directory(bytes, locator)?
    } else {
        if count == u16::MAX || size == u32::MAX || offset == u32::MAX {
            return Err(malformed("ZIP64 end record is missing"));
        }
        (u64::from(count), u64::from(size), u64::from(offset), end)
    };
    // ASVS 5.2.3: bound original entry counts before zip-rs's metadata allocation.
    if count > MAX_ENTRY_COUNT as u64 {
        return Err(bound_error(ZIP_INPUT, "archive has too many entries"));
    }
    let size = usize::try_from(size).map_err(|_| malformed("ZIP directory size is too large"))?;
    let start = directory_end
        .checked_sub(size)
        .ok_or_else(|| malformed("ZIP directory exceeds input"))?;
    if offset > start as u64 {
        return Err(malformed("ZIP directory offset exceeds input"));
    }
    let directory = Directory {
        count: count as usize,
        start,
    };
    check_records(bytes, &directory, directory_end)?;
    check_fallback_bounds(bytes)?;
    Ok(directory)
}

fn check_fallback_bounds(bytes: &[u8]) -> Result<(), Violation> {
    // zip-rs can retry any earlier EOCD if decoding the final directory fails.
    // Its comments need only fit inside the whole input, not reach its end.
    // Bound every candidate before it can allocate a metadata vector; validating
    // only our selected end record leaves the library's fallback unbounded.
    // ASVS 5.2.3: index candidates once; repeated prefix scans let small inputs
    // consume quadratic work even when every fallback directory is harmless.
    let mut latest_central = None;
    let mut hazards = Vec::new();
    let mut queries = Vec::new();
    let mut zip32_bound = false;
    for (end, signature) in bytes.windows(4).enumerate() {
        if signature == b"PK\x01\x02" {
            latest_central = Some(end);
        } else if signature == b"PK\x06\x06"
            && let Some(hazard) = zip64_hazard(bytes, end)
        {
            hazards.push(hazard);
        }
        if signature != b"PK\x05\x06" {
            continue;
        }
        let Ok(comment) = read_u16(bytes, end + 20) else {
            continue;
        };
        if end + 22 + usize::from(comment) > bytes.len() {
            continue;
        }
        let count = read_u16(bytes, end + 10)?;
        let size = read_u32(bytes, end + 12)?;
        let offset = read_u32(bytes, end + 16)?;
        let locator = end
            .checked_sub(20)
            .filter(|&position| bytes.get(position..position + 4) == Some(b"PK\x06\x07"));
        if (count == u16::MAX || size == u32::MAX || offset == u32::MAX)
            && let Some(locator) = locator
        {
            let declared_offset = read_u64(bytes, locator + 8)?;
            // These conditions stop zip-rs before it parses a ZIP64 candidate.
            if declared_offset < locator as u64 && read_u32(bytes, locator + 16)? <= 1 {
                queries.push((declared_offset as usize, locator));
            }
        } else if usize::from(count) > MAX_ENTRY_COUNT
            && u64::from(offset) < end as u64
            && latest_central
                .is_some_and(|position| position >= offset as usize && position + 4 <= end)
        {
            // Keep the original first-failing EOCD ordering: earlier ZIP64
            // queries must still run before this ZIP32 bound is returned.
            zip32_bound = true;
            break;
        }
    }
    hazards.sort_unstable_by_key(|hazard| hazard.end);
    let mut pending = hazards.into_iter().peekable();
    let mut eligible = BTreeMap::new();
    for (declared_offset, locator) in queries {
        while pending.peek().is_some_and(|hazard| hazard.end <= locator) {
            let hazard = pending.next().expect("eligible ZIP64 candidate");
            eligible.insert(hazard.start, hazard.message);
        }
        // Locators occur in ascending order. The tree contains exactly the
        // candidates ending before this locator; its first start in the range
        // matches the original ascending scan, including its violation message.
        if let Some((_, message)) = eligible.range(declared_offset..).next() {
            return Err(bound_error(ZIP_INPUT, message));
        }
    }
    if zip32_bound {
        Err(bound_error(ZIP_INPUT, "archive has too many entries"))
    } else {
        Ok(())
    }
}

struct Zip64Hazard {
    start: usize,
    end: usize,
    message: &'static str,
}

fn zip64_hazard(bytes: &[u8], position: usize) -> Option<Zip64Hazard> {
    let record_size = read_u64(bytes, position + 4).ok()?;
    // zip-rs reads the fixed header before checking the sector size. Even a
    // candidate later rejected for a locator mismatch can allocate its sector.
    // The fixed header may extend past a locator for a malformed 40-byte record.
    if bytes.get(position..position + 56).is_none() || record_size < 40 {
        return None;
    }
    let end = record_size.checked_add(position as u64 + 12)?;
    if end > bytes.len() as u64 {
        return None;
    }
    let message = if record_size.saturating_sub(44) > MAX_PACKAGE_BYTES {
        "ZIP64 metadata exceeds configured inspection bounds"
    } else if read_u64(bytes, position + 32).ok()? > MAX_ENTRY_COUNT as u64 {
        "archive has too many entries"
    } else {
        return None;
    };
    Some(Zip64Hazard {
        start: position,
        end: end as usize,
        message,
    })
}

fn zip64_directory(bytes: &[u8], locator: usize) -> Result<(u64, u64, u64, usize), Violation> {
    if read_u32(bytes, locator + 4)? != 0 || read_u32(bytes, locator + 16)? != 1 {
        return Err(malformed("multi-disk ZIP64 packages are unsupported"));
    }
    let declared_offset = read_u64(bytes, locator + 8)?;
    // The locator's offset is relative to the archive, which may have a prefix.
    // Match the actual end-record length to the locator to support ZIP64 prefixes
    // and extensible data without trusting a potentially overflowing offset.
    let end = bytes[..locator]
        .windows(4)
        .enumerate()
        .rev()
        .find(|&(position, signature)| {
            signature == b"PK\x06\x06"
                && position as u64 >= declared_offset
                && read_u64(bytes, position + 4).is_ok_and(|size| {
                    size >= 44 && size.checked_add(position as u64 + 12) == Some(locator as u64)
                })
        })
        .map(|(position, _)| position)
        .ok_or_else(|| malformed("missing or truncated ZIP64 end record"))?;
    if read_u32(bytes, end + 16)? != 0 || read_u32(bytes, end + 20)? != 0 {
        return Err(malformed("multi-disk ZIP64 packages are unsupported"));
    }
    let count = read_u64(bytes, end + 32)?;
    if read_u64(bytes, end + 24)? != count {
        return Err(malformed("ZIP64 entry counts disagree"));
    }
    Ok((
        count,
        read_u64(bytes, end + 40)?,
        read_u64(bytes, end + 48)?,
        end,
    ))
}

fn check_records(bytes: &[u8], directory: &Directory, end: usize) -> Result<(), Violation> {
    let mut names = BTreeSet::new();
    let mut position = directory.start;
    for _ in 0..directory.count {
        let header = bytes
            .get(position..position.saturating_add(46))
            .filter(|_| position.saturating_add(46) <= end)
            .ok_or_else(|| malformed("truncated central directory entry"))?;
        if &header[..4] != b"PK\x01\x02" {
            return Err(malformed("invalid central directory entry"));
        }
        let name_size = read_u16(header, 28)? as usize;
        let extra_size = read_u16(header, 30)? as usize;
        let comment_size = read_u16(header, 32)? as usize;
        let name_start = position + 46;
        let next = name_start
            .checked_add(name_size + extra_size + comment_size)
            .filter(|&next| next <= end)
            .ok_or_else(|| malformed("central directory entry exceeds input"))?;
        let name = &bytes[name_start..name_start + name_size];
        // Validate original names too: Unicode extra fields can replace the name
        // returned by zip-rs, while another ZIP consumer may use this raw field.
        validate_entry_name(&String::from_utf8_lossy(name), name.ends_with(b"/"))?;
        if !names.insert(name) {
            return Err(duplicate_error(&String::from_utf8_lossy(name)));
        }
        position = next;
    }
    // The optional central-directory digital signature is metadata, not a file.
    if position != end {
        let signature = bytes.get(position..end).unwrap_or_default();
        if !signature.starts_with(b"PK\x05\x05")
            || read_u16(signature, 4)? as usize + 6 != signature.len()
        {
            return Err(malformed("central directory count or size mismatch"));
        }
    }
    Ok(())
}

fn read_array<const N: usize>(bytes: &[u8], offset: usize) -> Result<[u8; N], Violation> {
    bytes
        .get(offset..offset.saturating_add(N))
        .and_then(|bytes| bytes.try_into().ok())
        .ok_or_else(|| malformed("truncated ZIP metadata"))
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16, Violation> {
    Ok(u16::from_le_bytes(read_array(bytes, offset)?))
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, Violation> {
    Ok(u32::from_le_bytes(read_array(bytes, offset)?))
}

fn read_u64(bytes: &[u8], offset: usize) -> Result<u64, Violation> {
    Ok(u64::from_le_bytes(read_array(bytes, offset)?))
}

fn malformed(message: &str) -> Violation {
    input_error(ZIP_INPUT, message)
}
