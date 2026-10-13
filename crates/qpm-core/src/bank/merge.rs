//! Bank combination and kind compatibility.

use super::{BankError, ItemBank};

impl ItemBank {
    /// Merges two banks, keeping the left order and replacing duplicate CRC values from the right.
    pub fn merge(&self, other: &Self) -> Result<Self, BankError> {
        let allow_mixed = self.allow_mixed || other.allow_mixed;
        validate_merged_kinds(self, other, allow_mixed)?;

        let mut items = self.items.clone();
        for (crc, item) in &other.items {
            items.insert(*crc, item.clone());
        }

        Ok(Self { allow_mixed, items })
    }
}

fn validate_merged_kinds(
    left: &ItemBank,
    right: &ItemBank,
    allow_mixed: bool,
) -> Result<(), BankError> {
    if allow_mixed {
        return Ok(());
    }
    let Some(allowed) = left.items.first().map(|(_, item)| item.kind()) else {
        return Ok(());
    };
    if let Some(attempted) = right.items.first().map(|(_, item)| item.kind())
        && allowed != attempted
    {
        return Err(BankError::MixedItemKinds { allowed, attempted });
    }
    Ok(())
}
