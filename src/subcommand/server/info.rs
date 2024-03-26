use super::*;
use axum::Json;
use shadow_rs::shadow;
use utoipa::{IntoParams, ToSchema};
shadow!(build);

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct NodeInfo {
  /// Node version of the API endpoint build.
  pub version: Option<String>,
  /// The name of the branch or tag of the API endpoint build.
  pub branch: Option<String>,
  /// Git commit hash of the API endpoint build.
  pub commit_hash: Option<String>,
  /// Build time of the API endpoint.
  pub build_time: Option<String>,
  /// Chain information of the blockchain.
  pub chain_info: ChainInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ChainInfo {
  /// The network of the blockchain.
  pub network: Option<String>,
  /// The block height of our indexer.
  #[schema(format = "uint32")]
  pub ord_block_height: u32,
  /// The block hash of our indexer.
  pub ord_block_hash: String,
  /// The chain block height of the blockchain.
  #[schema(format = "uint64")]
  pub chain_block_height: Option<u32>,
  /// The chain block hash of the blockchain.
  pub chain_block_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, IntoParams)]
pub struct NodeInfoQuery {
  /// Optional to query the BTC chain status.
  btc: Option<bool>,
}

/// Retrieve the indexer status.
///
/// Display indexer synchronization information, including indexer version, blockchain network, indexer height, blockchain network height, and other information.
#[utoipa::path(
    get,
    path = "/api/v1/node/info",
    params(
        NodeInfoQuery
  ),
    responses(
      (status = 200, description = "Obtain node runtime status.", body = Node),
      (status = 500, description = "Internal server error.", body = ApiError, example = json!(&ApiError::internal("internal error"))),
    )
  )]
pub(crate) async fn node_info(
  Extension(index): Extension<Arc<Index>>,
  Query(query): Query<NodeInfoQuery>,
) -> ApiResult<NodeInfo> {
  log::debug!("rpc: get node_info");
  let rtx = index.begin_read()?;
  let client = index.bitcoin_rpc_client()?;

  let (latest_height, latest_blockhash) = rtx.latest_block()?.ok_or_api_err(|| {
    ApiError::Internal("Failed to retrieve the latest block from the database.".to_string())
  })?;

  let (chain_block_height, chain_block_hash) = match query.btc.unwrap_or_default() {
    true => {
      let chain_blockchain_info = client.get_blockchain_info().map_err(ApiError::internal)?;
      (
        Some(u32::try_from(chain_blockchain_info.blocks).unwrap()),
        Some(chain_blockchain_info.best_block_hash),
      )
    }
    false => (None, None),
  };

  Ok(Json(ApiResponse::ok(NodeInfo {
    version: Some(build::PKG_VERSION.into()),
    branch: Some(build::BRANCH.into()),
    commit_hash: Some(build::SHORT_COMMIT.into()),
    build_time: Some(build::BUILD_TIME.into()),
    chain_info: ChainInfo {
      network: Some(index.get_chain().to_string()),
      ord_block_height: latest_height.0,
      ord_block_hash: latest_blockhash.to_string(),
      chain_block_height,
      chain_block_hash: chain_block_hash.map(|hash| hash.to_string()),
    },
  })))
}
