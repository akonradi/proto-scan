use std::convert::Infallible as Never;
use std::marker::PhantomData;

use prost::Message;
use proto_lens::read::Read;
use proto_lens::wire::GroupOp;
use proto_lens::wire::{
    FieldNumber, LengthDelimited, ParseEvent, ParseEventReader, ScalarField, ScalarWireType, Varint,
};
use proto_lens_tests::proto;

fn example_msg() -> proto::ScanExample {
    proto::ScanExample {
        repeated_msg: vec![proto::MultiFieldMessage {
            id: 1,
            name: "ABC".to_string(),
        }],
        single_msg: Some(proto::MultiFieldMessage {
            name: "a".to_owned(),
            id: 2,
        }),
        repeated_bool: vec![true, true, false, false],
        single_bool: Some(true),
        oneof_group: Some(proto::scan_example::OneofGroup::OneofFixed32(11111111)),
    }
}

pub trait ScanMessage {
    type Scanner;

    fn scanner() -> Self::Scanner;
}

pub trait Scan {
    type Output;

    fn next(&mut self) -> Option<Result<Self::Output, StopScan>>;
}

trait ScanCallbacks {
    type ScanEvent;

    fn on_scalar(
        &mut self,
        field: FieldNumber,
        value: ScalarField,
    ) -> Result<Self::ScanEvent, StopScan>;

    fn on_group(&mut self, field: FieldNumber, op: GroupOp) -> Result<Self::ScanEvent, StopScan>;

    fn on_length_delimited(
        &mut self,
        field: FieldNumber,
        delimited: impl LengthDelimited,
    ) -> Result<Self::ScanEvent, StopScan>;
}

struct ScanImpl<P, S>(P, S);

impl<P: ParseEventReader, S: ScanCallbacks> Scan for ScanImpl<P, S> {
    type Output = S::ScanEvent;

    fn next(&mut self) -> Option<Result<S::ScanEvent, StopScan>> {
        let Self(parse, fields) = self;
        let (field_number, event) = match parse.next() {
            Some(Err(_)) => return Some(Err(StopScan)),
            None => return None,
            Some(Ok(event)) => event,
        };

        let output = match event {
            ParseEvent::Scalar(scalar_field) => fields.on_scalar(field_number, scalar_field),
            ParseEvent::Group(group_op) => fields.on_group(field_number, group_op),
            ParseEvent::LengthDelimited(l) => fields.on_length_delimited(field_number, l),
        };
        Some(output)
    }
}

#[derive(Debug)]
pub struct StopScan;

pub struct ScanSingleImpl<T>(T);

/// Implemented by a visitor for a fixed [`FieldNumber`].
pub trait OnScanField {
    type ScanEvent;

    fn on_scalar(&mut self, value: ScalarField) -> Result<Option<Self::ScanEvent>, StopScan>;

    fn on_group(&mut self, op: GroupOp) -> Result<Option<Self::ScanEvent>, StopScan>;

    fn on_length_delimited(
        &mut self,
        delimited: impl LengthDelimited,
    ) -> Result<Option<Self::ScanEvent>, StopScan>;
}

/// Invokes the provided callback for each scalar value.
///
/// [`OnScanField::on_scalar`] returns an error if the encoded value has the
/// wrong wire type.
pub struct InvokeOn<W: ScalarWireType, F>(F, PhantomData<W>);

impl<'a, W: ScalarWireType, F: FnMut(W::Repr) -> Result<(), StopScan>> OnScanField
    for InvokeOn<W, F>
{
    type ScanEvent = Never;

    fn on_scalar(&mut self, value: ScalarField) -> Result<Option<Never>, StopScan> {
        let value = W::from_value(value).ok_or(StopScan)?;
        let () = (self.0)(value)?;
        Ok(None)
    }
    fn on_group(&mut self, _: GroupOp) -> Result<Option<Never>, StopScan> {
        Ok(None)
    }
    fn on_length_delimited(&mut self, _: impl LengthDelimited) -> Result<Option<Never>, StopScan> {
        Ok(None)
    }
}

/// [`OnScanField`] impl that does nothing and always succeeds.
#[derive(Default)]
pub struct NoOp;

impl OnScanField for NoOp {
    type ScanEvent = Never;

    fn on_scalar(&mut self, _value: ScalarField) -> Result<Option<Self::ScanEvent>, StopScan> {
        Ok(None)
    }

    fn on_group(&mut self, _op: GroupOp) -> Result<Option<Self::ScanEvent>, StopScan> {
        Ok(None)
    }

    fn on_length_delimited(
        &mut self,
        _delimited: impl LengthDelimited,
    ) -> Result<Option<Self::ScanEvent>, StopScan> {
        Ok(None)
    }
}

#[derive(Default)]
pub struct ScanExampleScan<A = NoOp, B = NoOp, C = NoOp, D = NoOp> {
    repeated_msg: A,
    single_msg: B,
    repeated_bool: C,
    single_bool: D,
}

impl ScanMessage for proto::ScanExample {
    type Scanner = ScanExampleScan<NoOp, NoOp, NoOp, NoOp>;

    fn scanner() -> Self::Scanner {
        ScanExampleScan::default()
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum ScanEvent4<E0, E1, E2, E3> {
    Event0(E0),
    Event1(E1),
    Event2(E2),
    Event3(E3),
}

impl<A: OnScanField, B: OnScanField, C: OnScanField, D: OnScanField> ScanExampleScan<A, B, C, D> {
    fn scan<'r>(
        self,
        read: impl Read + 'r,
    ) -> impl Scan<Output = <Self as ScanCallbacks>::ScanEvent> + 'r
    where
        A: 'r,
        B: 'r,
        C: 'r,
        D: 'r,
    {
        ScanImpl(proto_lens::wire::parse(read), self)
    }
}

impl<A: OnScanField, B: OnScanField, C: OnScanField, D: OnScanField> ScanCallbacks
    for ScanExampleScan<A, B, C, D>
{
    type ScanEvent = Option<ScanEvent4<A::ScanEvent, B::ScanEvent, C::ScanEvent, D::ScanEvent>>;

    fn on_scalar(
        &mut self,
        field: FieldNumber,
        value: ScalarField,
    ) -> Result<Self::ScanEvent, StopScan> {
        Ok(match u32::from(field) {
            1 => self.repeated_msg.on_scalar(value)?.map(ScanEvent4::Event0),
            2 => self.single_msg.on_scalar(value)?.map(ScanEvent4::Event1),
            3 => self.repeated_bool.on_scalar(value)?.map(ScanEvent4::Event2),
            4 => self.single_bool.on_scalar(value)?.map(ScanEvent4::Event3),
            _ => None,
        })
    }

    fn on_group(&mut self, field: FieldNumber, op: GroupOp) -> Result<Self::ScanEvent, StopScan> {
        Ok(match u32::from(field) {
            1 => self.repeated_msg.on_group(op)?.map(ScanEvent4::Event0),
            2 => self.single_msg.on_group(op)?.map(ScanEvent4::Event1),
            3 => self.repeated_bool.on_group(op)?.map(ScanEvent4::Event2),
            4 => self.single_bool.on_group(op)?.map(ScanEvent4::Event3),
            _ => None,
        })
    }

    fn on_length_delimited(
        &mut self,
        field: FieldNumber,
        delimited: impl LengthDelimited,
    ) -> Result<Self::ScanEvent, StopScan> {
        Ok(match u32::from(field) {
            1 => self
                .repeated_msg
                .on_length_delimited(delimited)?
                .map(ScanEvent4::Event0),
            2 => self
                .single_msg
                .on_length_delimited(delimited)?
                .map(ScanEvent4::Event1),
            3 => self
                .repeated_bool
                .on_length_delimited(delimited)?
                .map(ScanEvent4::Event2),
            4 => self
                .single_bool
                .on_length_delimited(delimited)?
                .map(ScanEvent4::Event3),
            _ => None,
        })
    }
}

impl<A, B, C> ScanExampleScan<A, B, C, NoOp> {
    pub fn save_single_bool<'t>(
        self,
        to: &'t mut impl From<bool>,
    ) -> ScanExampleScan<A, B, C, impl OnScanField<ScanEvent = Never> + 't> {
        let Self {
            repeated_msg,
            single_msg,
            repeated_bool,
            single_bool: NoOp,
        } = self;
        ScanExampleScan {
            repeated_msg,
            single_msg,
            repeated_bool,
            single_bool: SaveField::<'t, bool, _>(to, PhantomData),
        }
    }

    pub fn emit_single_bool(self) -> ScanExampleScan<A, B, C, impl OnScanField<ScanEvent = bool>> {
        let Self {
            repeated_msg,
            single_msg,
            repeated_bool,
            single_bool: NoOp,
        } = self;
        ScanExampleScan {
            repeated_msg,
            single_msg,
            repeated_bool,
            single_bool: EmitScalarField::<Varint, bool>(PhantomData),
        }
    }
}

pub struct EmitScalarField<W, T>(PhantomData<(W, T)>);

impl OnScanField for EmitScalarField<Varint, bool> {
    type ScanEvent = bool;

    fn on_scalar(&mut self, value: ScalarField) -> Result<Option<bool>, StopScan> {
        match value {
            ScalarField::Varint(0) => Ok(Some(false)),
            ScalarField::Varint(1) => Ok(Some(true)),
            _ => Err(StopScan),
        }
    }

    fn on_group(&mut self, _op: GroupOp) -> Result<Option<bool>, StopScan> {
        Err(StopScan)
    }

    fn on_length_delimited(
        &mut self,
        _delimited: impl LengthDelimited,
    ) -> Result<Option<bool>, StopScan> {
        Err(StopScan)
    }
}

pub struct SaveField<'t, T, D>(&'t mut D, PhantomData<T>);

impl<'t, D: From<bool>> OnScanField for SaveField<'t, bool, D> {
    type ScanEvent = Never;

    fn on_scalar(&mut self, value: ScalarField) -> Result<Option<Never>, StopScan> {
        match value {
            ScalarField::Varint(0) => *self.0 = false.into(),
            ScalarField::Varint(1) => *self.0 = true.into(),
            _ => return Err(StopScan),
        }
        Ok(None)
    }

    fn on_group(&mut self, _op: GroupOp) -> Result<Option<Never>, StopScan> {
        Err(StopScan)
    }

    fn on_length_delimited(
        &mut self,
        _delimited: impl LengthDelimited,
    ) -> Result<Option<Never>, StopScan> {
        Err(StopScan)
    }
}

mod save_single_bool {
    use super::*;

    #[test]
    fn empty() {
        let mut save_to = None;

        let scanner = proto::ScanExample::scanner().save_single_bool(&mut save_to);
        {
            let mut scan = scanner.scan([].as_slice());
            while let Some(event) = scan.next() {
                match event.unwrap() {
                    None => {}
                }
            }
        }

        assert_eq!(save_to, None);
    }

    #[test]
    fn with_field() {
        let mut save_to = None;

        let scanner = proto::ScanExample::scanner().save_single_bool(&mut save_to);
        {
            let message = example_msg().encode_to_vec();
            let mut scan = scanner.scan(message.as_slice());
            while let Some(event) = scan.next() {
                match event.unwrap() {
                    None => {}
                }
            }
        }

        assert_eq!(save_to, example_msg().single_bool);
    }
}

mod emit_single_bool {
    use super::*;

    #[test]
    fn empty() {
        let mut save_to = None;

        let scanner = proto::ScanExample::scanner().emit_single_bool();
        {
            let mut scan = scanner.scan([].as_slice());
            while let Some(event) = scan.next() {
                match event.unwrap() {
                    Some(ScanEvent4::Event3(b)) => save_to = Some(b),
                    None => {}
                }
            }
        }

        assert_eq!(save_to, None);
    }

    #[test]
    fn with_field() {
        let mut save_to = None;

        let scanner = proto::ScanExample::scanner().emit_single_bool();
        {
            let message = example_msg().encode_to_vec();
            let mut scan = scanner.scan(message.as_slice());
            while let Some(event) = scan.next() {
                match event.unwrap() {
                    Some(ScanEvent4::Event3(b)) => save_to = Some(b),
                    None => {}
                }
            }
        }

        assert_eq!(save_to, example_msg().single_bool);
    }
}
