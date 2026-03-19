use crate::scan::group::error::{GroupStackCapacity, WrongGroupId};

/// A stack of field numbers corresponding to open groups.
///
/// Each SGROUP tag encountered begins a new group, identified by the field
/// number for the tag. The group is ended by a corresponding EGROUP tag with
/// the same field number. Per the protobuf specification, corresponding start
/// and end tags must be correctly nested for a protobuf wire format to be
/// valid.
pub trait GroupStack {
    /// Pushes a new field number onto the stack if there is space.
    fn push(&mut self, field_number: u32) -> Result<(), GroupStackCapacity>;

    // Pops the last field number on the stack and returns an error if it
    // doesn't match the argument.
    fn pop(&mut self, field_number: u32) -> Result<(), WrongGroupId>;
}

impl GroupStack for () {
    fn push(&mut self, _field_number: u32) -> Result<(), GroupStackCapacity> {
        Err(GroupStackCapacity)
    }
    fn pop(&mut self, _field_number: u32) -> Result<(), WrongGroupId> {
        Err(WrongGroupId)
    }
}

impl<const N: usize> GroupStack for arrayvec::ArrayVec<u32, N> {
    fn push(&mut self, field_number: u32) -> Result<(), GroupStackCapacity> {
        arrayvec::ArrayVec::try_push(self, field_number).map_err(|_| GroupStackCapacity)
    }

    fn pop(&mut self, field_number: u32) -> Result<(), WrongGroupId> {
        if arrayvec::ArrayVec::pop(self) == Some(field_number) {
            Ok(())
        } else {
            Err(WrongGroupId)
        }
    }
}

#[cfg(feature = "std")]
impl GroupStack for Vec<u32> {
    fn push(&mut self, field_number: u32) -> Result<(), GroupStackCapacity> {
        Vec::push(self, field_number);
        Ok(())
    }

    fn pop(&mut self, field_number: u32) -> Result<(), WrongGroupId> {
        if self.pop() == Some(field_number) {
            Ok(())
        } else {
            Err(WrongGroupId)
        }
    }
}
