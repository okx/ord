use crate::index::Rtx;
use serde::{Deserialize, Serialize};
use {
  super::{error::ApiError, *},
  axum::Json,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "type")]
pub enum ApiContentEncoding {
  Br { decode: String },
  Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiInscription {
  pub id: InscriptionId,
  pub number: i32,
  pub content_type: Option<String>,
  pub content: Option<String>,
  pub content_length: Option<usize>,
  pub content_encoding: Option<ApiContentEncoding>,
  pub metadata: Option<String>,
  pub metaprotocol: Option<String>,
  pub parents: Vec<u32>,
  pub delegate: Option<InscriptionId>,
  pub pointer: Option<u64>,
  pub owner: Option<ApiUtxoAddress>,
  pub genesis_height: u32,
  pub genesis_timestamp: u32,
  pub location: SatPoint,
  // pub collections: Vec<String>,
  pub charms: Vec<Charm>,
  pub sat: Option<u64>,
}

// /ord/id/:id/inscription
/// Retrieve the inscription infomation with the specified inscription id.

pub(crate) async fn ord_inscription_id(
  Extension(settings): Extension<Arc<Settings>>,
  Extension(index): Extension<Arc<Index>>,
  Path(id): Path<String>,
) -> ApiResult<ApiInscription> {
  log::debug!("rpc: get ord_inscription_id: {id}");

  task::block_in_place(|| {
    let rtx = index.begin_read()?;
    let inscription_id = InscriptionId::from_str(&id).map_err(ApiError::bad_request)?;

    let sequence_number = Index::sequence_number_by_inscription_id(inscription_id, &rtx)?
      .ok_or(OrdApiError::InscriptionNotFoundById(inscription_id))?;

    ord_inscription_by_sequence_number(sequence_number, &rtx, &index, &settings)
  })
}

// /ord/number/:number/inscription
/// Retrieve the inscription infomation with the specified inscription number.
pub(crate) async fn ord_inscription_number(
  Extension(settings): Extension<Arc<Settings>>,
  Extension(index): Extension<Arc<Index>>,
  Path(number): Path<i32>,
) -> ApiResult<ApiInscription> {
  log::debug!("rpc: get ord_inscription_number: {number}");

  task::block_in_place(|| {
    let rtx = index.begin_read()?;
    let sequence_number = Index::sequence_number_by_inscription_number(number, &rtx)?
      .ok_or(OrdApiError::InscriptionNotFoundByNum(number))?;

    ord_inscription_by_sequence_number(sequence_number, &rtx, &index, &settings)
  })
}

fn ord_inscription_by_sequence_number(
  sequence_number: u32,
  rtx: &Rtx,
  index: &Index,
  settings: &Settings,
) -> ApiResult<ApiInscription> {
  log::debug!("rpc: get ord_inscription_by_sequence_number: {sequence_number}");

  let inscription_entry = Index::inscription_entry_by_sequence_number(sequence_number, rtx)?
    .ok_or(OrdApiError::InscriptionEntryNotFound(sequence_number))?;

  let transaction = Index::get_tx(inscription_entry.id.txid, rtx, index)?
    .ok_or(OrdApiError::TransactionNotFound(inscription_entry.id.txid))?;

  let inscription = ParsedEnvelope::from_transaction(&transaction)
    .into_iter()
    .nth(inscription_entry.id.index.try_into().unwrap())
    .map(|envelope| envelope.payload)
    .ok_or(OrdApiError::ParsedEnvelopeError(
      inscription_entry.id.index.try_into().unwrap(),
      inscription_entry.id.txid,
    ))?;

  let location = Index::location_by_sequence_number(sequence_number, rtx)?
    .ok_or(OrdApiError::LocationNotFound(sequence_number))?;

  let owner = if location.outpoint.txid != Hash::all_zeros() {
    let transaction = if location.outpoint.txid != inscription_entry.id.txid {
      Index::get_tx(location.outpoint.txid, &rtx, &index)?
        .ok_or(OrdApiError::TransactionNotFound(location.outpoint.txid))?
    } else {
      transaction
    };
    Some(ApiUtxoAddress::from(UtxoAddress::from_script(
      &transaction
        .output
        .into_iter()
        .nth(location.outpoint.vout.try_into().unwrap())
        .unwrap()
        .script_pubkey,
      &settings.chain(),
    )))
  } else {
    None
  };

  Ok(Json(ApiResponse::ok(ApiInscription {
    id: inscription_entry.id,
    number: inscription_entry.inscription_number,
    content_type: inscription.content_type().map(str::to_string),
    content: inscription.body().map(hex::encode),
    content_length: inscription.content_length(),
    content_encoding: decompress_encoding_body(&inscription),
    metaprotocol: inscription.metaprotocol().map(str::to_string),
    metadata: inscription
      .metadata()
      .and_then(|_| inscription.metadata.as_deref().map(hex::encode)),
    parents: inscription_entry.parents,
    pointer: inscription.pointer(),
    delegate: inscription.delegate(),
    owner,
    genesis_height: inscription_entry.height,
    genesis_timestamp: inscription_entry.timestamp,
    location,
    // collections: collections.iter().map(|c| c.to_string()).collect(),
    charms: Charm::charms(inscription_entry.charms),
    sat: inscription_entry.sat.map(|s| s.0),
  })))
}

fn decompress_encoding_body(inscription: &Inscription) -> Option<ApiContentEncoding> {
  if let Some(header_value) = inscription.content_encoding() {
    if header_value == "br" {
      if let Some(body) = inscription.body() {
        let mut decompressed = Vec::new();
        if Decompressor::new(body, 4096)
          .read_to_end(&mut decompressed)
          .is_ok()
        {
          return Some(ApiContentEncoding::Br {
            decode: hex::encode(decompressed),
          });
        }
      }
    }
    return Some(ApiContentEncoding::Unknown);
  }
  None
}

#[cfg(test)]
mod tests {
  use super::*;
  use brotli::{
    enc::{backward_references::BrotliEncoderMode, BrotliEncoderParams},
    CompressorWriter,
  };
  use std::io::Write;

  #[test]
  fn test_serialize_ord_inscription() {
    let mut ord_inscription = ApiInscription {
      id: InscriptionId {
        txid: txid(1),
        index: 0xFFFFFFFF,
      },
      number: -100,
      content_type: Some("content_type".to_string()),
      content: Some("content".to_string()),
      content_length: Some("content".to_string().len()),
      content_encoding: Some(ApiContentEncoding::Br {
        decode: "content_encoding".to_string(),
      }),
      metaprotocol: Some("mata_protocol".to_string()),
      metadata: Some("0123456789abcdef".to_string()),
      parents: vec![1],
      delegate: Some(InscriptionId {
        txid: txid(1),
        index: 0xFFFFFFFD,
      }),
      pointer: Some(0),
      owner: Some(
        UtxoAddress::from_script(
          &Address::from_str("bc1qhvd6suvqzjcu9pxjhrwhtrlj85ny3n2mqql5w4")
            .unwrap()
            .assume_checked()
            .script_pubkey(),
          &Chain::Mainnet,
        )
        .into(),
      ),
      genesis_height: 1,
      genesis_timestamp: 100,
      location: SatPoint::from_str(
        "5660d06bd69326c18ec63127b37fb3b32ea763c3846b3334c51beb6a800c57d3:1:3000",
      )
      .unwrap(),
      // collections: Vec::new(),
      charms: vec![Charm::Vindicated],
      sat: None,
    };
    assert_eq!(
      serde_json::to_string_pretty(&ord_inscription).unwrap(),
      r#"{
  "id": "1111111111111111111111111111111111111111111111111111111111111111i4294967295",
  "number": -100,
  "contentType": "content_type",
  "content": "content",
  "contentLength": 7,
  "contentEncoding": {
    "type": "br",
    "decode": "content_encoding"
  },
  "metadata": "0123456789abcdef",
  "metaprotocol": "mata_protocol",
  "parents": [
    1
  ],
  "delegate": "1111111111111111111111111111111111111111111111111111111111111111i4294967293",
  "pointer": 0,
  "owner": {
    "address": "bc1qhvd6suvqzjcu9pxjhrwhtrlj85ny3n2mqql5w4"
  },
  "genesisHeight": 1,
  "genesisTimestamp": 100,
  "location": "5660d06bd69326c18ec63127b37fb3b32ea763c3846b3334c51beb6a800c57d3:1:3000",
  "charms": [
    "vindicated"
  ],
  "sat": null
}"#,
    );
    ord_inscription.owner = None;
    assert_eq!(
      serde_json::to_string_pretty(&ord_inscription).unwrap(),
      r#"{
  "id": "1111111111111111111111111111111111111111111111111111111111111111i4294967295",
  "number": -100,
  "contentType": "content_type",
  "content": "content",
  "contentLength": 7,
  "contentEncoding": {
    "type": "br",
    "decode": "content_encoding"
  },
  "metadata": "0123456789abcdef",
  "metaprotocol": "mata_protocol",
  "parents": [
    1
  ],
  "delegate": "1111111111111111111111111111111111111111111111111111111111111111i4294967293",
  "pointer": 0,
  "owner": null,
  "genesisHeight": 1,
  "genesisTimestamp": 100,
  "location": "5660d06bd69326c18ec63127b37fb3b32ea763c3846b3334c51beb6a800c57d3:1:3000",
  "charms": [
    "vindicated"
  ],
  "sat": null
}"#,
    );
  }

  #[test]
  fn test_decompress_encoding_body() {
    let mut compressed = Vec::new();
    let body = "ord".as_bytes();

    CompressorWriter::with_params(
      &mut compressed,
      body.len(),
      &BrotliEncoderParams {
        lgblock: 24,
        lgwin: 24,
        mode: BrotliEncoderMode::BROTLI_MODE_TEXT,
        quality: 11,
        size_hint: body.len(),
        ..Default::default()
      },
    )
    .write_all(body)
    .unwrap();

    let inscription = Inscription {
      content_encoding: Some("br".as_bytes().to_vec()),
      ..inscription("text/plain;charset=utf-8", compressed)
    };
    assert_eq!(
      decompress_encoding_body(&inscription),
      Some(ApiContentEncoding::Br {
        decode: hex::encode(body)
      })
    );
  }

  #[test]
  fn test_except_decompress_encoding_body() {
    let body = "ord".as_bytes();

    let inscription1 = Inscription {
      content_encoding: Some("br".as_bytes().to_vec()),
      ..inscription("text/plain;charset=utf-8", body)
    };
    assert_eq!(
      decompress_encoding_body(&inscription1),
      Some(ApiContentEncoding::Unknown)
    );
    let body = Vec::new();

    let inscription2 = Inscription {
      content_encoding: Some("br".as_bytes().to_vec()),
      ..inscription("text/plain;charset=utf-8", body)
    };
    assert_eq!(
      decompress_encoding_body(&inscription2),
      Some(ApiContentEncoding::Unknown)
    );
  }

  #[test]
  fn test_serialize_content_encoding() {
    assert_eq!(
      serde_json::to_string(&ApiContentEncoding::Br {
        decode: "content_encoding".to_string(),
      })
      .unwrap(),
      r#"{"type":"br","decode":"content_encoding"}"#
    );
    assert_eq!(
      serde_json::to_string(&ApiContentEncoding::Unknown).unwrap(),
      r#"{"type":"unknown"}"#
    );
  }
}
