use {
  super::{error::ApiError, types::ScriptPubkey, *},
  crate::okx::datastore::ScriptKey,
  axum::Json,
  utoipa::ToSchema,
};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[schema(as = ord::ApiInscriptionDigest)]
#[serde(rename_all = "camelCase")]
pub struct ApiInscriptionDigest {
  /// The inscription id.
  pub id: String,
  /// The inscription number.
  pub number: i32,
  /// The inscription location.
  pub location: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[schema(as = ord::ApiOutPointResult)]
#[serde(rename_all = "camelCase")]
pub struct ApiOutPointResult {
  #[schema(value_type = Option<ord::ApiOutpointInscriptions>)]
  pub result: Option<ApiOutpointInscriptions>,
  pub latest_blockhash: String,
  #[schema(format = "uint64")]
  pub latest_height: u32,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[schema(as = ord::ApiOutpointInscriptions)]
#[serde(rename_all = "camelCase")]
pub struct ApiOutpointInscriptions {
  /// The transaction id.
  pub txid: String,
  /// The script pubkey.
  pub script_pub_key: String,
  /// The owner of the script pubkey.
  pub owner: ScriptPubkey,
  /// The value of the transaction output.
  #[schema(format = "uint64")]
  pub value: u64,
  #[schema(value_type = Vec<ord::ApiInscriptionDigest>)]
  /// The inscriptions on the transaction output.
  pub inscription_digest: Vec<ApiInscriptionDigest>,
}

// /ord/outpoint/:outpoint/info
/// Retrieve the outpoint infomation with the specified outpoint.
#[utoipa::path(
  get,
  path = "/api/v1/ord/outpoint/{outpoint}/info",
  params(
      ("outpoint" = String, Path, description = "Outpoint")
),
  responses(
    (status = 200, description = "Obtain outpoint infomation", body = OrdOutPointData),
    (status = 400, description = "Bad query.", body = ApiError, example = json!(&ApiError::bad_request("bad request"))),
    (status = 404, description = "Not found.", body = ApiError, example = json!(&ApiError::not_found("not found"))),
    (status = 500, description = "Internal server error.", body = ApiError, example = json!(&ApiError::internal("internal error"))),
  )
)]
pub(crate) async fn ord_outpoint(
  Extension(index): Extension<Arc<Index>>,
  Path(outpoint): Path<OutPoint>,
) -> ApiResult<ApiOutPointResult> {
  log::debug!("rpc: get ord_outpoint: {outpoint}");

  let rtx = index.begin_read()?;

  let (latest_height, latest_blockhash) = rtx.latest_block()?.ok_or_api_err(|| {
    OrdApiError::Internal("Failed to retrieve the latest block from the database.".to_string())
      .into()
  })?;

  let inscriptions_with_satpoints = rtx.inscriptions_on_output_with_satpoints(outpoint)?;
  let chain = index.get_chain();

  // If there are no inscriptions on the output, return None and parsed block states.
  if inscriptions_with_satpoints.is_empty() {
    return Ok(Json(ApiResponse::ok(ApiOutPointResult {
      result: None,
      latest_height: latest_height.n(),
      latest_blockhash: latest_blockhash.to_string(),
    })));
  }

  let mut inscription_digests = Vec::with_capacity(inscriptions_with_satpoints.len());
  for (satpoint, inscription_id) in inscriptions_with_satpoints {
    inscription_digests.push(ApiInscriptionDigest {
      id: inscription_id.to_string(),
      number: rtx
        .get_inscription_entry(inscription_id)?
        .map(|inscription_entry| inscription_entry.inscription_number)
        .ok_or(OrdApiError::UnknownInscriptionId(inscription_id))?,
      location: satpoint.to_string(),
    });
  }

  // Get the txout from the database store or from an RPC request.
  let vout = Index::fetch_vout(
    &rtx,
    &index.bitcoin_rpc_client()?,
    outpoint,
    chain,
    index.has_transactions_index(),
  )?
  .ok_or(OrdApiError::TransactionNotFound(outpoint.txid))?;

  Ok(Json(ApiResponse::ok(ApiOutPointResult {
    result: Some(ApiOutpointInscriptions {
      txid: outpoint.txid.to_string(),
      script_pub_key: vout.script_pubkey.to_asm_string(),
      owner: ScriptKey::from_script(&vout.script_pubkey, chain).into(),
      value: vout.value,
      inscription_digest: inscription_digests,
    }),
    latest_height: latest_height.n(),
    latest_blockhash: latest_blockhash.to_string(),
  })))
}
