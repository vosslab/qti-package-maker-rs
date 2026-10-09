//! Text input and per-block warning locations for text2qti.

use qti_core::{ItemBank, MemoryAssets};

use super::NAME;
use super::media::restore_markdown_images;
use super::parser::{parse_block, split_questions};
use crate::{EngineError, ReadInput, ReadLocation, ReadOutcome, ReadWarning, Reader};

pub(super) struct Text2QtiReader;

impl Reader for Text2QtiReader {
    fn name(&self) -> &'static str {
        NAME
    }

    fn read_items(
        &self,
        input: ReadInput<'_>,
        allow_mixed: bool,
    ) -> Result<ReadOutcome, EngineError> {
        let text = input.text(NAME)?;
        // The reader recognizes only this engine's Markdown image form and restores it once at
        // the input boundary. Attribute encoding occurs here, immediately before HTML creation
        // (ASVS 1.1.1, 1.1.2, 1.2.1).
        let restored = restore_markdown_images(text);
        let mut bank = ItemBank::new(allow_mixed);
        let mut warnings = Vec::new();
        for (number, block) in split_questions(&restored).into_iter().enumerate() {
            match parse_block(&block) {
                Ok(Some(item)) => match bank.add_item(item) {
                    Ok(_) => {}
                    Err(error) => warnings.push(block_warning(number + 1, error.to_string())),
                },
                Ok(None) => warnings.push(block_warning(
                    number + 1,
                    "unrecognized question block skipped".to_owned(),
                )),
                Err(message) => warnings.push(block_warning(number + 1, message)),
            }
        }
        Ok(ReadOutcome {
            bank,
            assets: MemoryAssets::new(),
            warnings,
        })
    }
}

fn block_warning(number: usize, message: String) -> ReadWarning {
    ReadWarning {
        location: ReadLocation::Block { number },
        message,
    }
}
