# Media handling

Image handling is centralized in `qti-core::media`. Item HTML remains authored content. The media
layer scans `<img>` sources only at a conversion boundary, resolves each source against the
bank's media base, assigns collision-safe output names, and supplies a writer-owned render view.
It never mutates the validated source `Item` or its CRC.

## Asset sources

An image source is classified as local, external, or data URI. Local sources resolve beneath an
external input directory or a bank-owned temporary directory. Resolution and writes normalize
author-controlled paths, confine them beneath the media base, and reject unsafe symlink escapes.
The bank keeps temporary media alive through clones and derived banks while any owner still needs
it.

The library deduplicates exact source references for resolution and records each item's media
dependencies in document order. Package names are derived after full-bank collection so equal
basenames from different sources cannot overwrite each other.

## Writer policies

Every registry entry declares one media policy. `apply_media_policy` returns decisions and
provenance-complete warnings with engine name, item CRC, authored source, resolved path, action,
and reason.

| Policy | Local images | External URLs | Data URIs |
| --- | --- | --- | --- |
| `Package` | copy/rewrite into package or inline self-test output | keep verbatim and warn | package-file writers reject; self-test retains authored data URI |
| `ReferenceWarn` | retain reference and warn | retain reference and warn | retain reference and warn |
| `PlaceholderWarn` | replace with readable placeholder and warn | replace with readable placeholder and warn | replace with readable placeholder and warn |
| `Fail` | reject if any image is present | reject | reject |

`PlaceholderWarn` uses `[image: name]`, preferring an assigned package name, then `embedded image`
for data URIs, then the source basename without its query string. The human-readable writer keeps
a related description order: placeholder, authored alt text, then source path.

## Package writers

Canvas QTI 1.2, Blackboard QTI 2.1, and Blackboard Original package local image bytes and rewrite
their output HTML to their package locations. SVG local images package with a warning because LMS
support is not guaranteed. External URLs stay authored and do not trigger a network request.

Before output creation, writers collect and resolve their assets. ZIP assembly preflights all
bytes before replacing a destination, so unreadable media or a rejected data URI cannot leave a
partial package. See [FORMATS.md](FORMATS.md) for each engine's format behavior and
[PARITY.md](PARITY.md) for package-integrity evidence.

## HTML-to-image conversion

The shared conversion pass carries forward the source bank's media base and creates a separate
bank-owned temporary root only after all fragment renders succeed. Generated PNGs are then added
to that derived bank. This two-phase sequence keeps a renderer failure from creating a media
directory or changing the source bank. See [HTML_TO_IMAGE.md](HTML_TO_IMAGE.md).

