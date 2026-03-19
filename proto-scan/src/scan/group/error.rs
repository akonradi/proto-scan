#![doc(hidden)]

#[cfg(doc)]
use super::GroupStack;
use crate::scan::ScanError;

/// Error returned when a [`GroupStack`] implementation runs out of room.
pub struct GroupStackCapacity;

impl<R> From<GroupStackCapacity> for ScanError<R> {
    fn from(GroupStackCapacity: GroupStackCapacity) -> Self {
        Self::GroupOverflow
    }
}

/// Error returned when an SGROUP or EGROUP tag is found to not have a
/// corresponding partner.
pub struct WrongGroupId;

impl<R> From<WrongGroupId> for ScanError<R> {
    fn from(WrongGroupId: WrongGroupId) -> Self {
        ScanError::GroupMismatch
    }
}
