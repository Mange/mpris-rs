//! The module containing the metadata related types.
#[allow(clippy::module_inception)]
mod metadata;
mod track_id;
mod values;

pub use self::metadata::{Metadata, MetadataIntoIter, RawMetadata};
pub use self::track_id::TrackID;
pub use self::values::MetadataValue;
