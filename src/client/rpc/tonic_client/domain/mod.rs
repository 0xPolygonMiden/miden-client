use core::any::type_name;

use crate::errors::ConversionError;

pub mod accounts;
pub mod blocks;
pub mod digest;
pub mod merkle;
pub mod notes;
pub mod nullifiers;
pub mod transactions;

// UTILITIES
// ================================================================================================

pub trait MissingFieldHelper {
    fn missing_field(field_name: &'static str) -> ConversionError;
}

impl<T: prost::Message> MissingFieldHelper for T {
    fn missing_field(field_name: &'static str) -> ConversionError {
        ConversionError::MissingFieldInProtobufRepresentation {
            entity: type_name::<T>(),
            field_name,
        }
    }
}
