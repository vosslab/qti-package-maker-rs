//! Blackboard Original pool-export ZIP reader and writer.
//!
//! Question XML lives in pool `.dat` entries; images use Blackboard's csfiles
//! transport. Reader results recover image bytes without host filesystem access.

mod read;
mod write;

use qti_core::ItemKind;
use qti_core::media::{AssetSource, MediaPolicy};

use crate::{EngineError, ReadInput, ReadOutcome, Reader, WriteContext, WriteOutcome, Writer};

pub(crate) const NAME: &str = "blackboard_export_zip";

/// Creates the Blackboard pool-export writer.
pub fn boxed_writer() -> Box<dyn Writer> {
    Box::new(BlackboardWriter)
}

/// Creates the Blackboard pool-export reader.
pub fn boxed_reader() -> Box<dyn Reader> {
    Box::new(BlackboardReader)
}

struct BlackboardWriter;

impl Writer for BlackboardWriter {
    fn name(&self) -> &'static str {
        NAME
    }
    fn media_policy(&self) -> MediaPolicy {
        crate::engine(NAME)
            .expect("registered Blackboard engine")
            .media_policy
    }
    fn supported_kinds(&self) -> &'static [ItemKind] {
        crate::engine(NAME)
            .expect("registered Blackboard engine")
            .supported_kinds
    }
    fn write_package(
        &self,
        bank: &qti_core::ItemBank,
        assets: &dyn AssetSource,
        context: &WriteContext,
    ) -> Result<WriteOutcome, EngineError> {
        write::write_package(bank, assets, context)
    }
}

struct BlackboardReader;

impl Reader for BlackboardReader {
    fn name(&self) -> &'static str {
        NAME
    }
    fn read_items(
        &self,
        input: ReadInput<'_>,
        allow_mixed: bool,
    ) -> Result<ReadOutcome, EngineError> {
        read::read_package(input, allow_mixed)
    }
}
