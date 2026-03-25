use prost::Message as _;
use proto_scan::read::ReadTypes;
use proto_scan::scan::field::map::{MapEntry, MapEntryScanner};
use proto_scan::scan::field::{MapKey, RepeatStrategy, RepeatStrategyScanner, Save, ScanRepeated};
use proto_scan::scan::{
    IntoScanOutput, IntoScanner, ScanCallbacks, ScanDelimited, ScanError, ScanMessage as _,
    ScannerBuilder as _,
};
use test_case::test_case;

use super::*;
use InputKind::*;

#[test_case(Empty)]
#[test_case(Full)]
fn scan_message(input: InputKind) {
    let input = input.into_map_message();
    let bytes = input.encode_to_vec();

    let scanner = proto::WithMap::scanner()
        .fixed64_to_i32(Save)
        .fixed64_to_message(Save::with_value(proto::MapValue::scanner().id(Save)))
        .string_to_i32(Save)
        .string_to_message(Save::with_value(proto::MapValue::scanner().id(Save)));

    let output = scanner.scan(&mut bytes.as_slice()).read_all().unwrap();

    let expected = proto::ScanWithMapOutput::<_, _, _, _> {
        fixed64_to_i32: input.fixed64_to_i32,
        fixed64_to_message: input
            .fixed64_to_message
            .into_iter()
            .map(|(k, v)| (k, proto::ScanMapValueOutput { id: v.id }))
            .collect(),
        string_to_i32: input
            .string_to_i32
            .iter()
            .map(|(k, v)| (k.as_str(), *v))
            .collect(),
        string_to_message: input
            .string_to_message
            .iter()
            .map(|(k, v)| (k.as_str(), proto::ScanMapValueOutput { id: v.id }))
            .collect(),
    };
    assert_eq!(output, expected)
}

#[test_case(Empty)]
#[test_case(Full)]
fn scan_as_repeated(input: InputKind) {
    const STR_KEY: &str = "message";
    const FIXED64_KEY: u64 = 2;
    assert_ne!(Full.into_map_message().string_to_message.get(STR_KEY), None);
    assert_ne!(
        Full.into_map_message().fixed64_to_i32.get(&FIXED64_KEY),
        None
    );

    let input = input.into_map_message();
    let bytes = input.encode_to_vec();

    let scanner = proto::WithMap::scanner()
        .string_to_message(
            MapEntry::scanner()
                .key(Save)
                .value(proto::MapValue::scanner().id(Save))
                .repeat_by(KeepLastForKey(STR_KEY)),
        )
        .fixed64_to_i32(
            MapEntry::scanner()
                .key(Save)
                .value(Save)
                .repeat_by(KeepLastForKey(FIXED64_KEY)),
        );

    let output = scanner.scan(&mut bytes.as_slice()).read_all().unwrap();
    let expected = proto::ScanWithMapOutput {
        string_to_message: input
            .string_to_message
            .get(STR_KEY)
            .map(|v| proto::ScanMapValueOutput { id: v.id }),
        fixed64_to_i32: input.fixed64_to_i32.get(&FIXED64_KEY).copied(),
        ..Default::default()
    };

    assert_eq!(output, expected);
}

struct KeepLastForKey<K>(K);

struct KeepLastForKeyImpl<K, O>(K, Option<O>);

impl<K: MapKey + ?Sized, V: ?Sized, KK, SV: IntoScanner<V>>
    RepeatStrategy<MapEntryScanner<K, V, Save, SV>> for KeepLastForKey<KK>
{
    type Impl<R: ReadTypes> =
        KeepLastForKeyImpl<KK, <SV::Scanner<R> as IntoScanOutput>::ScanOutput>;

    fn into_impl<R: ReadTypes>(self) -> Self::Impl<R> {
        KeepLastForKeyImpl(self.0, None)
    }
}

impl<
    's,
    KK,
    K: PartialEq<KK>,
    V,
    R: ReadTypes,
    S: ScanCallbacks<R> + IntoScanOutput<ScanOutput = (K, V)> + Clone,
> RepeatStrategyScanner<R, S> for KeepLastForKeyImpl<KK, V>
{
    fn on_message(
        &mut self,
        scanner: &S,
        input: impl ScanDelimited<ReadTypes = R>,
    ) -> Result<(), ScanError<<R as ReadTypes>::Error>> {
        let mut scanner = scanner.clone();
        input.scan_with(&mut scanner)?;
        let (k, v) = scanner.into_scan_output();
        if k == self.0 {
            self.1 = Some(v)
        }
        Ok(())
    }
}

impl<K, O> IntoScanOutput for KeepLastForKeyImpl<K, O> {
    type ScanOutput = Option<O>;
    fn into_scan_output(self) -> Self::ScanOutput {
        self.1
    }
}
