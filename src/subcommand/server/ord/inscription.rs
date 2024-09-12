use {
  super::{error::ApiError, types::ScriptPubkey, *},
  crate::{index::rtx::Rtx, okx::datastore::ScriptKey},
  axum::Json,
  utoipa::ToSchema,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
#[schema(as = ord::ApiContentEncoding)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "type")]
pub enum ApiContentEncoding {
  Br { decode: String },
  Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[schema(as = ord::ApiInscription)]
#[serde(rename_all = "camelCase")]
pub struct ApiInscription {
  /// The inscription id.
  pub id: String,
  /// The inscription number.
  pub number: i32,
  /// The inscription content type.
  pub content_type: Option<String>,
  /// The inscription content body.
  pub content: Option<String>,
  /// The inscription content body length.
  pub content_length: Option<usize>,
  /// Decode the content encoding if the message has a content encoding tag.
  pub content_encoding: Option<ApiContentEncoding>,
  /// The inscription metadata.
  pub metadata: Option<String>,
  /// The inscription metaprotocol.
  pub metaprotocol: Option<String>,
  /// The inscription parent inscription id.
  pub parent: Option<InscriptionId>,
  /// The delegate inscription id of the inscription.
  pub delegate: Option<InscriptionId>,
  /// The inscription pointer.
  pub pointer: Option<u64>,
  /// The inscription owner.
  pub owner: Option<ScriptPubkey>,
  /// The inscription genesis block height.
  #[schema(format = "uint32")]
  pub genesis_height: u32,
  /// The inscription genesis timestamp.
  #[schema(format = "uint32")]
  pub genesis_timestamp: u32,
  /// The inscription location.
  pub location: String,
  /// Collections of Inscriptions.
  pub collections: Vec<String>,
  /// Charms of Inscriptions.
  pub charms: Vec<String>,
  /// The inscription sat index.  
  pub sat: Option<u64>,
}

// /ord/id/:id/inscription
/// Retrieve the inscription infomation with the specified inscription id.
#[utoipa::path(
  get,
  path = "/api/v1/ord/id/{id}/inscription",
  params(
      ("id" = String, Path, description = "inscription ID")
),
  responses(
    (status = 200, description = "Obtain inscription infomation.", body = OrdOrdInscription),
    (status = 400, description = "Bad query.", body = ApiError, example = json!(&ApiError::bad_request("bad request"))),
    (status = 404, description = "Not found.", body = ApiError, example = json!(&ApiError::not_found("not found"))),
    (status = 500, description = "Internal server error.", body = ApiError, example = json!(&ApiError::internal("internal error"))),
  )
)]
pub(crate) async fn ord_inscription_id(
  Extension(index): Extension<Arc<Index>>,
  Path(id): Path<String>,
) -> ApiResult<ApiInscription> {
  log::debug!("rpc: get ord_inscription_id: {id}");

  let rtx = index.begin_read()?;
  let chain = index.get_chain();
  let client = index.bitcoin_rpc_client()?;
  let index_transactions = index.has_transactions_index();

  let id = InscriptionId::from_str(&id).map_err(ApiError::bad_request)?;

  ord_get_inscription_by_id(id, &rtx, client, chain, index_transactions)
}

// /ord/number/:number/inscription
/// Retrieve the inscription infomation with the specified inscription number.
#[utoipa::path(
  get,
  path = "/api/v1/ord/number/{number}/inscription",
  params(
      ("number" = i64, Path, description = "inscription number")
),
  responses(
    (status = 200, description = "Obtain inscription infomation.", body = OrdOrdInscription),
    (status = 400, description = "Bad query.", body = ApiError, example = json!(&ApiError::bad_request("bad request"))),
    (status = 404, description = "Not found.", body = ApiError, example = json!(&ApiError::not_found("not found"))),
    (status = 500, description = "Internal server error.", body = ApiError, example = json!(&ApiError::internal("internal error"))),
  )
)]
pub(crate) async fn ord_inscription_number(
  Extension(index): Extension<Arc<Index>>,
  Path(number): Path<i32>,
) -> ApiResult<ApiInscription> {
  log::debug!("rpc: get ord_inscription_number: {number}");

  let rtx = index.begin_read()?;
  let chain = index.get_chain();
  let client = index.bitcoin_rpc_client()?;
  let index_transactions = index.has_transactions_index();

  let inscription_id = Index::get_inscription_id_by_inscription_number_with_rtx(number, &rtx)?
    .ok_or(OrdApiError::UnknownInscriptionNumber(number))?;

  ord_get_inscription_by_id(inscription_id, &rtx, client, chain, index_transactions)
}

fn ord_get_inscription_by_id(
  inscription_id: InscriptionId,
  rtx: &Rtx,
  client: Client,
  chain: Chain,
  index_transactions: bool,
) -> ApiResult<ApiInscription> {
  let inscription_entry = Index::get_inscription_entry_with_rtx(inscription_id, rtx)?
    .ok_or(OrdApiError::UnknownInscriptionId(inscription_id))?;

  let tx =
    Index::get_transaction_with_rtx(inscription_id.txid, rtx, &client, chain, index_transactions)?
      .ok_or(OrdApiError::TransactionNotFound(inscription_id.txid))?;

  let inscription = ParsedEnvelope::from_transaction(&tx)
    .get(usize::try_from(inscription_id.index).unwrap())
    .map(|envelope: &ParsedEnvelope| envelope.payload.clone())
    .ok_or(OrdApiError::InvalidInscription(inscription_id))?;

  let sat_point = Index::get_inscription_satpoint_by_id_with_rtx(inscription_id, rtx)?
    .ok_or(OrdApiError::SatPointNotFound(inscription_id))?;

  let collections = rtx
    .ord_inscription_id_to_collections(inscription_id)?
    .unwrap_or_default();

  let parent_inscription_id = match inscription_entry.parent {
    Some(parent) => rtx
      .sequence_number_to_inscription_entry(parent)?
      .map(|entry| entry.id),
    None => None,
  };

  let charms: Vec<Charm> = Charm::ALL
    .iter()
    .filter(|charm| charm.is_set(inscription_entry.charms))
    .cloned()
    .collect();

  let location_outpoint = sat_point.outpoint;

  let output = if location_outpoint == unbound_outpoint() {
    None
  } else {
    let location_transaction = if tx.txid() != location_outpoint.txid {
      Index::get_transaction_with_rtx(
        location_outpoint.txid,
        rtx,
        &client,
        chain,
        index_transactions,
      )?
      .ok_or(OrdApiError::TransactionNotFound(location_outpoint.txid))?
    } else {
      tx.clone()
    };
    location_transaction
      .output
      .into_iter()
      .nth(location_outpoint.vout.try_into().unwrap())
  };

  Ok(Json(ApiResponse::ok(ApiInscription {
    id: inscription_id.to_string(),
    number: inscription_entry.inscription_number,
    content_type: inscription.content_type().map(str::to_string),
    content: inscription.body().map(hex::encode),
    content_length: inscription.content_length(),
    content_encoding: decompress_encoding_body(&inscription),
    metaprotocol: inscription.metaprotocol().map(str::to_string),
    metadata: inscription
      .metadata()
      .and_then(|_| inscription.metadata.as_deref().map(hex::encode)),
    parent: parent_inscription_id,
    pointer: inscription.pointer(),
    delegate: inscription.delegate(),
    owner: output.map(|vout| ScriptKey::from_script(&vout.script_pubkey, chain).into()),
    genesis_height: inscription_entry.height,
    genesis_timestamp: inscription_entry.timestamp,
    location: sat_point.to_string(),
    collections: collections.iter().map(|c| c.to_string()).collect(),
    charms: charms.iter().map(|c| c.title().into()).collect(),
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

// ord/debug/bitmap/district/:number
pub(crate) async fn ord_debug_bitmap_district(
  Extension(index): Extension<Arc<Index>>,
  Path(number): Path<u32>,
) -> ApiResult<InscriptionId> {
  log::debug!("rpc: get ord_debug_bitmap_district: number:{}", number);

  let rtx = index.begin_read()?;
  let inscription_id = rtx
    .ord_district_to_inscription_id(number)?
    .ok_or_api_not_found(format!("district {number} not found."))?;

  log::debug!(
    "rpc: get ord_debug_bitmap_district: {:?} {:?}",
    number,
    inscription_id
  );

  Ok(Json(ApiResponse::ok(inscription_id)))
}

// ord/debug/btc_nam/:btc_name
pub(crate) async fn ord_debug_btc_name(
  Extension(index): Extension<Arc<Index>>,
  Path(btc_name): Path<String>,
) -> ApiResult<InscriptionId> {
  log::debug!("rpc: get ord_debug_btc_name:{btc_name}");

  let rtx = index.begin_read()?;
  let inscription_id = rtx
    .btc_name_to_inscription_id(&btc_name, &index.domain_list)?
    .ok_or_api_not_found(format!("district {btc_name} not found."))?;

  log::debug!(
    "rpc: get ord_debug_bitmap_district: {:?} {:?}",
    btc_name,
    inscription_id
  );

  Ok(Json(ApiResponse::ok(inscription_id)))
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
      }
      .to_string(),
      number: -100,
      content_type: Some("content_type".to_string()),
      content: Some("content".to_string()),
      content_length: Some("content".to_string().len()),
      content_encoding: Some(ApiContentEncoding::Br {
        decode: "content_encoding".to_string(),
      }),
      metaprotocol: Some("mata_protocol".to_string()),
      metadata: Some("0123456789abcdef".to_string()),
      parent: Some(InscriptionId {
        txid: txid(1),
        index: 0xFFFFFFFE,
      }),
      delegate: Some(InscriptionId {
        txid: txid(1),
        index: 0xFFFFFFFD,
      }),
      pointer: Some(0),
      owner: Some(
        ScriptKey::from_script(
          &Address::from_str("bc1qhvd6suvqzjcu9pxjhrwhtrlj85ny3n2mqql5w4")
            .unwrap()
            .assume_checked()
            .script_pubkey(),
          Chain::Mainnet,
        )
        .into(),
      ),
      genesis_height: 1,
      genesis_timestamp: 100,
      location: SatPoint::from_str(
        "5660d06bd69326c18ec63127b37fb3b32ea763c3846b3334c51beb6a800c57d3:1:3000",
      )
      .unwrap()
      .to_string(),
      collections: Vec::new(),
      charms: [Charm::Vindicated]
        .iter()
        .map(|c| c.title().into())
        .collect(),
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
  "parent": "1111111111111111111111111111111111111111111111111111111111111111i4294967294",
  "delegate": "1111111111111111111111111111111111111111111111111111111111111111i4294967293",
  "pointer": 0,
  "owner": {
    "address": "bc1qhvd6suvqzjcu9pxjhrwhtrlj85ny3n2mqql5w4"
  },
  "genesisHeight": 1,
  "genesisTimestamp": 100,
  "location": "5660d06bd69326c18ec63127b37fb3b32ea763c3846b3334c51beb6a800c57d3:1:3000",
  "collections": [],
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
  "parent": "1111111111111111111111111111111111111111111111111111111111111111i4294967294",
  "delegate": "1111111111111111111111111111111111111111111111111111111111111111i4294967293",
  "pointer": 0,
  "owner": null,
  "genesisHeight": 1,
  "genesisTimestamp": 100,
  "location": "5660d06bd69326c18ec63127b37fb3b32ea763c3846b3334c51beb6a800c57d3:1:3000",
  "collections": [],
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
