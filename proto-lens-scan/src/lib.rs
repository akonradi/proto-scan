use std::convert::Infallible as Never;
use std::marker::PhantomData;

pub use proto_lens::read::Read;
pub use proto_lens::wire::{
    FieldNumber, GroupOp, I32, I64, LengthDelimited, ScalarField, Varint, parse,
};
use proto_lens::wire::{ParseEvent, ParseEventReader, ScalarWireType};

pub trait ScanMessage {
    type Scanner;

    fn scanner() -> Self::Scanner;
}

pub trait Scan {
    type Event;
    type Output;

    fn next(&mut self) -> Option<Result<Self::Event, StopScan>>;

    fn read_all(self) -> Result<Self::Output, StopScan>;
}

pub trait ScanCallbacks {
    type ScanEvent;
    type ScanOutput: Default + Extend<Self::ScanEvent>;

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

pub struct ScanWith<P, S>(P, S);

impl<P, S> ScanWith<P, S> {
    pub fn new(input: P, scanner: S) -> Self {
        Self(input, scanner)
    }
}

impl<P: ParseEventReader, S: ScanCallbacks> Scan for ScanWith<P, S> {
    type Event = S::ScanEvent;
    type Output = S::ScanOutput;

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

    fn read_all(mut self) -> Result<Self::Output, StopScan> {
        let mut output = Self::Output::default();
        while let Some(event) = self.next() {
            output.extend(Some(event?));
        }
        Ok(output)
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

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ScanEvent4<E0, E1, E2, E3> {
    Event0(E0),
    Event1(E1),
    Event2(E2),
    Event3(E3),
}

impl<A: OnScanField, B: OnScanField, C: OnScanField, D: OnScanField> ScanExampleScan<A, B, C, D> {
    fn scan<'r>(
        self,
        read: impl Read + 'r,
    ) -> impl Scan<Event = <Self as ScanCallbacks>::ScanEvent> + 'r
    where
        A: 'r,
        B: 'r,
        C: 'r,
        D: 'r,
    {
        ScanWith(proto_lens::wire::parse(read), self)
    }
}

impl<A: OnScanField, B: OnScanField, C: OnScanField, D: OnScanField> ScanCallbacks
    for ScanExampleScan<A, B, C, D>
{
    type ScanEvent = Option<ScanEvent4<A::ScanEvent, B::ScanEvent, C::ScanEvent, D::ScanEvent>>;
    type ScanOutput = ScanExampleScan<
        Option<A::ScanEvent>,
        Option<B::ScanEvent>,
        Option<C::ScanEvent>,
        Option<D::ScanEvent>,
    >;

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

impl<A, B, C, D> Extend<Option<ScanEvent4<A, B, C, D>>>
    for ScanExampleScan<Option<A>, Option<B>, Option<C>, Option<D>>
{
    fn extend<T: IntoIterator<Item = Option<ScanEvent4<A, B, C, D>>>>(&mut self, iter: T) {
        let Self {
            repeated_msg,
            single_msg,
            repeated_bool,
            single_bool,
        } = self;
        for event in iter {
            let Some(event) = event else { continue };
            match event {
                ScanEvent4::Event0(e) => *repeated_msg = Some(e),
                ScanEvent4::Event1(e) => *single_msg = Some(e),
                ScanEvent4::Event2(e) => *repeated_bool = Some(e),
                ScanEvent4::Event3(e) => *single_bool = Some(e),
            }
        }
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
            single_bool: SaveField::<'t, Varint, bool, _>(to, PhantomData),
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

impl<W, T> EmitScalarField<W, T> {
    pub fn new() -> Self {
        Self(PhantomData)
    }
}

pub struct SaveField<'t, W, T, D>(&'t mut D, PhantomData<(W, T)>);

impl<'t, W, T, D> SaveField<'t, W, T, D> {
    pub fn new(to: &'t mut D) -> Self {
        Self(to, PhantomData)
    }
}

impl<'t, D: From<bool>> OnScanField for SaveField<'t, Varint, bool, D> {
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
