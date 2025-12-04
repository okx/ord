use {super::*, crate::okx::brc20::entry::OpiBlockValidation};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiOpiBlockValidation {
  pub block_hash: Option<BlockHash>,
  pub block_timestamp: Option<u32>,
  pub brc20_block_event_hash: Option<String>,
  pub brc20_cumulative_event_hash: Option<String>,
  pub brc20_prog_block_trace_hash: Option<String>,
  pub brc20_cumulative_trace_hash: Option<String>,
}

impl From<OpiBlockValidation> for ApiOpiBlockValidation {
  fn from(validation: OpiBlockValidation) -> Self {
    Self {
      block_hash: Some(validation.block_hash),
      block_timestamp: Some(validation.block_timestamp),
      brc20_block_event_hash: Some(validation.brc20_block_event_hash),
      brc20_cumulative_event_hash: Some(validation.brc20_cumulative_event_hash),
      brc20_prog_block_trace_hash: validation.brc20_prog_block_trace_hash,
      brc20_cumulative_trace_hash: validation.brc20_cumulative_trace_hash,
    }
  }
}

pub(crate) async fn brc20_opi_block_validation(
  Extension(index): Extension<Arc<Index>>,
  Path(height): Path<u32>,
) -> ApiResult<ApiOpiBlockValidation> {
  tracing::debug!("rpc: get brc20_opi_block_validation: {}", height);
  task::block_in_place(|| {
    let rtx = index.begin_read()?;
    let validation = trace_db_call!("get_opi_block_validation", {
      Index::brc20_get_opi_block_validation(height, &rtx)
    })?
    .map(|v| v.into())
    .unwrap_or_default();

    tracing::debug!(
      "rpc: get brc20_opi_block_validation: {:?} {:?}",
      height,
      validation
    );

    Ok(Json(ApiResponse::ok(validation)))
  })
}
