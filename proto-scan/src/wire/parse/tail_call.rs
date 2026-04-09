//! Tail-call interpreter for scanning.
//!
//! Utilizes the unstable `become` keyword to implement protobuf scanning as a
//! sequence of mutually recursive tail call functions.
//!
//! Based on the writeup in https://www.mattkeeter.com/blog/2026-04-05-tailcall/
use std::ops::Index;

use crate::DecodeError;
use crate::read::Read;
use crate::read::count_reader::CountReader;
use crate::scan::IntoScanOutput;
use crate::wire::parse::{
    DoBeforeNext, EventReader, LengthDelimitedImpl, LimitReader, ParseCallbacks,
};
use crate::wire::{
    FieldNumber, I32, I64, NumericField, NumericWireType as _, Tag, Varint, WireType,
};

#[allow(type_alias_bounds)]
type FnPtr<R: Read, P: ParseCallbacks<R::ReadTypes> + IntoScanOutput> =
    extern "rust-preserve-none" fn(R, P, FieldNumber) -> Result<P::ScanOutput, P::ParseError>;

/// Table of function pointers corresponding to [`WireType`] variants.
struct Table<F> {
    on_varint: F,
    on_i64: F,
    on_length_delimited: F,
    on_sgroup: F,
    on_egroup: F,
    on_i32: F,
}

impl<R: Read, P: ParseCallbacks<R::ReadTypes> + IntoScanOutput> Table<FnPtr<R, P>> {
    const fn new() -> Self {
        Self {
            on_varint: on_varint::<R, P> as FnPtr<R, P>,
            on_i64: on_i64::<R, P> as FnPtr<R, P>,
            on_i32: on_i32::<R, P> as FnPtr<R, P>,
            on_sgroup: on_sgroup::<R, P> as FnPtr<R, P>,
            on_egroup: on_egroup::<R, P> as FnPtr<R, P>,
            on_length_delimited: on_length_delimited::<R, P> as FnPtr<R, P>,
        }
    }
}

impl<F> Index<WireType> for Table<F> {
    type Output = F;
    fn index(&self, wire_type: WireType) -> &Self::Output {
        match wire_type {
            WireType::Varint => &self.on_varint,
            WireType::I64 => &self.on_i64,
            WireType::LengthDelimited => &self.on_length_delimited,
            WireType::Sgroup => &self.on_sgroup,
            WireType::Egroup => &self.on_egroup,
            WireType::I32 => &self.on_i32,
        }
    }
}

/// Helper for declaring tail-recursive functions.
macro_rules! define_callback {
    // Declares a fn with the given name.
    ($name:ident ( $reader:ident, $cb:ident, $field_number:ident ) $body:block) => {
        extern "rust-preserve-none" fn $name<R: Read, P: ParseCallbacks<R::ReadTypes> + IntoScanOutput>(
            mut $reader: R,
            mut $cb: P,
            $field_number: FieldNumber,
        ) -> Result<P::ScanOutput, P::ParseError> {
            $body;
            define_callback!(($reader, $cb), become);
        }
    };

    // Declares the tail of a function that reads the next tag and then dispatches.
    (( $reader:ident, $cb:ident ), $b:tt) => {
        let mut tag_reader = CountReader::new(&mut $reader);

        let tag = match Tag::read_from(&mut tag_reader) {
            Ok(tag) => tag,
            Err(DecodeError::UnexpectedEnd) if tag_reader.count() == 0 => return Ok($cb.into_scan_output()),
            Err(e) => return Err(e.into()),
        };

        let Tag {
            wire_type,
            field_number,
        } = tag;

        let f = Table::new()[wire_type];
        $b f($reader, $cb, field_number)
    };
}

define_callback!(on_varint (reader, cb, field_number) {
    let n = NumericField::Varint(Varint::read_from(&mut reader)?);
    cb.on_numeric(field_number, n)?;
});

define_callback!(on_i32 (reader, cb, field_number) {
    let n = NumericField::I32(I32::read_from(&mut reader)?);
    cb.on_numeric(field_number, n)?;
});
define_callback!(on_i64 (reader, cb, field_number) {
    let n = NumericField::I64(I64::read_from(&mut reader)?);
    cb.on_numeric(field_number, n)?;
});
define_callback!(on_sgroup( reader, cb, field_number) {
    cb.on_group_start(field_number, &mut EventReader::new(&mut reader))?;
});
define_callback!(on_egroup (reader, cb, field_number) {
    cb.on_group_end(field_number)?;
});
define_callback!(on_length_delimited ( reader, cb, field_number) {
    let length = Varint::read_from(&mut reader)?;
    let to_skip = u32::try_from(length).map_err(|_| DecodeError::TooLargeLengthDelimited(length))?;
    let mut do_next = DoBeforeNext::DoNothing;

    cb.on_length_delimited(
        field_number,
        LengthDelimitedImpl {
            reader: LimitReader::new(&mut reader, to_skip),
            write_back_to: &mut do_next,
        },
    )?;
    match do_next {
        DoBeforeNext::Skip(to_skip) => {
            reader.skip(to_skip.get()).map_err(DecodeError::from)?;
        }
        DoBeforeNext::DoNothing => {}
    }
});

/// Optimized implementation of [`ParseEventReader::read_all`](super::ParseEventReader::read_all).
///
/// Uses tail recursion and table-based dispatch on wire type to speed up parsing.
#[inline(always)]
pub(super) fn read_all<R: Read, S: ParseCallbacks<R::ReadTypes> + IntoScanOutput>(
    mut reader: R,
    scanner: S,
) -> Result<S::ScanOutput, S::ParseError> {
    define_callback!((reader, scanner), return);
}
