use super::*;
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeInfo {
  pub version: &'static str,
  pub branch: &'static str,
  pub commit_hash: &'static str,
  pub chain_info: ChainInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChainInfo {
  pub network: String,
  pub ord_block_height: u32,
  pub ord_block_hash: String,
  pub chain_block_height: Option<u32>,
  pub chain_block_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfoQuery {
  btc: Option<bool>,
}

pub async fn node_info(
  Extension(settings): Extension<Arc<Settings>>,
  Extension(index): Extension<Arc<Index>>,
  Query(query): Query<NodeInfoQuery>,
) -> ApiResult<NodeInfo> {
  log::debug!("rpc: get node_info");
  let rtx = index.begin_read()?;

  let (latest_height, latest_blockhash) = Index::latest_block(&rtx)?.ok_or_api_err(|| {
    ApiError::Internal("Failed to retrieve the latest block from the database.".to_string())
  })?;

  let (chain_block_height, chain_block_hash) = match query.btc.unwrap_or_default() {
    true => {
      let chain_blockchain_info = index
        .client
        .get_blockchain_info()
        .map_err(ApiError::internal)?;
      (
        Some(u32::try_from(chain_blockchain_info.blocks).unwrap()),
        Some(chain_blockchain_info.best_block_hash),
      )
    }
    false => (None, None),
  };

  Ok(Json(ApiResponse::ok(NodeInfo {
    version: env!("CARGO_PKG_VERSION"),
    branch: env!("GIT_BRANCH"),
    commit_hash: env!("GIT_COMMIT"),
    chain_info: ChainInfo {
      network: settings.chain().network().to_string(),
      ord_block_height: latest_height.0,
      ord_block_hash: latest_blockhash.to_string(),
      chain_block_height,
      chain_block_hash: chain_block_hash.map(|hash| hash.to_string()),
    },
  })))
}
