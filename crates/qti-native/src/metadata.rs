//! Resolve clock and invocation labels once before writer fan-out.

use qti_engines::{DocumentMetadata, WriteContext};

use crate::NativeError;

pub fn resolve_write_context(output_name: &str, title: &str) -> Result<WriteContext, NativeError> {
    let now = time::OffsetDateTime::now_local()
        .map_err(|error| NativeError::LocalDate(error.to_string()))?;
    Ok(WriteContext::new(
        output_name,
        DocumentMetadata {
            title: title.to_owned(),
            date: now.date().to_string(),
        },
        now.unix_timestamp_nanos() as u64,
    )?)
}
