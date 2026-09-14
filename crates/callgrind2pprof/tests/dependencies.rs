//! Tests serialization infrastructure, not the pprof schema or conversion.
use flate2::{Compression, read::GzDecoder, write::GzEncoder};
use prost::Message;
use std::io::{Read, Write};

#[derive(Clone, PartialEq, Message)]
struct SmokeMessage {
    #[prost(uint64, tag = "1")]
    instructions: u64,
}

#[test]
fn protobuf_round_trips_through_gzip_without_protoc_or_system_zlib() {
    let original = SmokeMessage { instructions: 42 };
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&original.encode_to_vec()).unwrap();
    let compressed = encoder.finish().unwrap();
    assert_eq!(&compressed[..2], &[0x1f, 0x8b]);
    let mut decoded = Vec::new();
    GzDecoder::new(compressed.as_slice())
        .read_to_end(&mut decoded)
        .unwrap();
    assert_eq!(SmokeMessage::decode(decoded.as_slice()).unwrap(), original);
}
