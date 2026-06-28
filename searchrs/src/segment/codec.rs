use crate::index::doctable::DocId;
use crate::index::memindex::Position;
use crate::segment::format::TermPostings;

use std::collections::HashMap;
use std::io;

pub trait PostingCodec: Send + Sync {
    fn encode(&self, postings: &HashMap<DocId, Vec<Position>>) -> io::Result<Vec<u8>>;

    fn decode(&self, bytes: &[u8]) -> io::Result<HashMap<DocId, Vec<Position>>>;

    fn clone_box(&self) -> Box<dyn PostingCodec>;
}

pub struct BincodePostingCodec;

impl PostingCodec for BincodePostingCodec {
    fn clone_box(&self) -> Box<dyn PostingCodec> {
        Box::new(BincodePostingCodec)
    }

    fn encode(&self, postings: &HashMap<DocId, Vec<Position>>) -> io::Result<Vec<u8>> {
        let docs: Vec<_> = postings
            .iter()
            .map(|(&doc_id, positions)| (doc_id, positions.clone()))
            .collect();

        bincode::serialize(&TermPostings { docs }).map_err(|err| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("serialize term postings: {err}"),
            )
        })
    }

    fn decode(&self, bytes: &[u8]) -> io::Result<HashMap<DocId, Vec<Position>>> {
        let postings: TermPostings = bincode::deserialize(bytes).map_err(|err| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("deserialize term postings: {err}"),
            )
        })?;

        Ok(postings.docs.into_iter().collect())
    }
}

pub struct CompressedPostingCodec;

impl PostingCodec for CompressedPostingCodec {
    fn clone_box(&self) -> Box<dyn PostingCodec> {
        Box::new(CompressedPostingCodec)
    }

    fn encode(&self, postings: &HashMap<DocId, Vec<Position>>) -> io::Result<Vec<u8>> {
        Ok(encode_postings(postings).bytes)
    }

    fn decode(&self, bytes: &[u8]) -> io::Result<HashMap<DocId, Vec<Position>>> {
        decode_postings(&CompressedPostingList {
            bytes: bytes.to_vec(),
        })
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "decode compressed postings"))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompressedPostingList {
    pub bytes: Vec<u8>,
}

pub fn encode_postings(postings: &HashMap<DocId, Vec<Position>>) -> CompressedPostingList {
    let mut docs: Vec<_> = postings.iter().collect();
    docs.sort_by_key(|entry| *entry.0);

    let doc_ids: Vec<u64> = docs.iter().map(|entry| *entry.0).collect();
    let doc_gaps = encode_doc_gaps(&doc_ids);

    let mut values = Vec::new();

    values.push(docs.len() as u64);

    for ((_, positions), doc_gap) in docs.into_iter().zip(doc_gaps) {
        values.push(doc_gap);
        values.push(positions.len() as u64);

        let pos_gaps = encode_position_gaps(positions);

        for gap in pos_gaps {
            values.push(gap);
        }
    }

    CompressedPostingList {
        bytes: encode_u64s(&values),
    }
}

pub fn decode_postings(list: &CompressedPostingList) -> Option<HashMap<DocId, Vec<Position>>> {
    let values = decode_u64s(&list.bytes)?;
    let mut offset = 0usize;

    let doc_count = *values.get(offset)? as usize;
    offset += 1;

    let mut postings = HashMap::new();
    let mut current_doc = 0u64;

    for _ in 0..doc_count {
        let doc_gap = *values.get(offset)?;
        offset += 1;

        current_doc += doc_gap;

        let tf = *values.get(offset)? as usize;
        offset += 1;

        let mut pos_gaps = Vec::with_capacity(tf);

        for _ in 0..tf {
            pos_gaps.push(*values.get(offset)?);
            offset += 1;
        }

        let positions = decode_position_gaps(&pos_gaps);

        postings.insert(current_doc, positions);
    }

    if offset != values.len() {
        return None;
    }

    Some(postings)
}

pub fn encode_doc_gaps(doc_ids: &[DocId]) -> Vec<DocId> {
    let mut prev = 0;
    let mut gaps = Vec::with_capacity(doc_ids.len());

    for &doc_id in doc_ids {
        gaps.push(doc_id - prev);
        prev = doc_id;
    }

    gaps
}

pub fn decode_doc_gaps(gaps: &[DocId]) -> Vec<DocId> {
    let mut current = 0;
    let mut doc_ids = Vec::with_capacity(gaps.len());

    for &gap in gaps {
        current += gap;
        doc_ids.push(current);
    }

    doc_ids
}

pub fn encode_position_gaps(positions: &[Position]) -> Vec<Position> {
    let mut prev = 0;
    let mut gaps = Vec::with_capacity(positions.len());

    for &pos in positions {
        gaps.push(pos - prev);
        prev = pos;
    }

    gaps
}

pub fn decode_position_gaps(gaps: &[Position]) -> Vec<Position> {
    let mut current = 0;
    let mut positions = Vec::with_capacity(gaps.len());

    for &gap in gaps {
        current += gap;
        positions.push(current);
    }

    positions
}

pub fn encode_varint(mut value: u64, out: &mut Vec<u8>) {
    while value >= 0x80 {
        out.push((value as u8) | 0x80);
        value >>= 7;
    }

    out.push(value as u8);
}

pub fn decode_varint(input: &[u8], offset: &mut usize) -> Option<u64> {
    let mut result = 0u64;
    let mut shift = 0;

    while *offset < input.len() {
        let byte = input[*offset];
        *offset += 1;

        result |= ((byte & 0x7F) as u64) << shift;

        if byte & 0x80 == 0 {
            return Some(result);
        }

        shift += 7;

        if shift >= 64 {
            return None;
        }
    }

    None
}

pub fn encode_u64s(values: &[u64]) -> Vec<u8> {
    let mut out = Vec::new();

    for &value in values {
        encode_varint(value, &mut out);
    }

    out
}

pub fn decode_u64s(input: &[u8]) -> Option<Vec<u64>> {
    let mut values = Vec::new();
    let mut offset = 0;

    while offset < input.len() {
        values.push(decode_varint(input, &mut offset)?);
    }

    Some(values)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn doc_gap_encoding_roundtrip() {
        let doc_ids = vec![1, 5, 9, 13, 20];

        let gaps = encode_doc_gaps(&doc_ids);
        let decoded = decode_doc_gaps(&gaps);

        assert_eq!(gaps, vec![1, 4, 4, 4, 7]);
        assert_eq!(decoded, doc_ids);
    }

    #[test]
    fn position_gap_encoding_roundtrip() {
        let positions = vec![0, 3, 7, 10];

        let gaps = encode_position_gaps(&positions);
        let decoded = decode_position_gaps(&gaps);

        assert_eq!(gaps, vec![0, 3, 4, 3]);
        assert_eq!(decoded, positions);
    }

    #[test]
    fn gap_encoding_handles_empty_inputs() {
        let doc_ids: Vec<DocId> = Vec::new();
        let positions: Vec<Position> = Vec::new();

        assert!(encode_doc_gaps(&doc_ids).is_empty());
        assert!(decode_doc_gaps(&doc_ids).is_empty());

        assert!(encode_position_gaps(&positions).is_empty());
        assert!(decode_position_gaps(&positions).is_empty());
    }

    #[test]
    fn varint_roundtrip_small_values() {
        let values = vec![0, 1, 2, 10, 127];

        let bytes = encode_u64s(&values);
        let decoded = decode_u64s(&bytes).unwrap();

        assert_eq!(decoded, values);
    }

    #[test]
    fn varint_roundtrip_large_values() {
        let values = vec![128, 255, 300, 16_384, 1_000_000];

        let bytes = encode_u64s(&values);
        let decoded = decode_u64s(&bytes).unwrap();

        assert_eq!(decoded, values);
    }

    #[test]
    fn varint_rejects_truncated_input() {
        let bytes = vec![0x80];

        assert_eq!(decode_u64s(&bytes), None);
    }

    #[test]
    fn varint_uses_fewer_bytes_for_small_numbers() {
        let small = encode_u64s(&[1, 2, 3, 4]);
        let raw_u64_bytes = 4 * std::mem::size_of::<u64>();

        assert!(small.len() < raw_u64_bytes);
    }

    #[test]
    fn compressed_postings_roundtrip() {
        let postings = HashMap::from([(1, vec![0, 2, 5]), (5, vec![1]), (9, vec![3, 7])]);

        let compressed = encode_postings(&postings);
        let decoded = decode_postings(&compressed).unwrap();

        assert_eq!(decoded, postings);
    }
    #[test]
    fn compressed_postings_handles_empty_postings() {
        let postings: HashMap<DocId, Vec<Position>> = HashMap::new();

        let compressed = encode_postings(&postings);
        let decoded = decode_postings(&compressed).unwrap();

        assert!(decoded.is_empty());
    }

    #[test]
    fn compressed_postings_rejects_truncated_bytes() {
        let postings = HashMap::from([(1, vec![0, 2, 5]), (5, vec![1])]);

        let mut compressed = encode_postings(&postings);
        compressed.bytes.pop();

        assert_eq!(decode_postings(&compressed), None);
    }

    #[test]
    fn bincode_posting_codec_roundtrip() {
        let postings = HashMap::from([(1, vec![0, 2, 5]), (5, vec![1]), (9, vec![3, 7])]);

        let codec = BincodePostingCodec;
        let bytes = codec.encode(&postings).unwrap();
        let decoded = codec.decode(&bytes).unwrap();

        assert_eq!(decoded, postings);
    }

    #[test]
    fn compressed_posting_codec_roundtrip() {
        let postings = HashMap::from([(1, vec![0, 2, 5]), (5, vec![1]), (9, vec![3, 7])]);

        let codec = CompressedPostingCodec;
        let bytes = codec.encode(&postings).unwrap();
        let decoded = codec.decode(&bytes).unwrap();

        assert_eq!(decoded, postings);
    }
}
