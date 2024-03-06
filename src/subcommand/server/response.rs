use {
  super::{info::NodeInfo, *},
  utoipa::ToSchema,
};
#[derive(Default, Debug, Clone, Serialize, Deserialize, ToSchema)]
#[aliases(
  ApiBRC20Tick = ApiResponse<brc20::ApiTickInfo>,
  ApiBRC20AllTick = ApiResponse<brc20::ApiTickInfos>,
  ApiBRC20Balance = ApiResponse<brc20::ApiBalance>,
  ApiBRC20AllBalance = ApiResponse<brc20::ApiBalances>,
  ApiBRC20TxEvents = ApiResponse<brc20::ApiTxEvents>,
  ApiBRC20BlockEvents = ApiResponse<brc20::ApiBlockEvents>,
  ApiBRC20Transferable = ApiResponse<brc20::ApiTransferableAssets>,

  ApiOrdInscription = ApiResponse<ord::ApiInscription>,
  ApiOrdOutPointData = ApiResponse<ord::ApiOutpointInscriptions>,
  ApiOrdOutPointResult = ApiResponse<ord::ApiOutPointResult>,
  ApiOrdTxInscriptions = ApiResponse<ord::ApiTxInscriptions>,
  ApiOrdBlockInscriptions = ApiResponse<ord::ApiBlockInscriptions>,

  Node = ApiResponse<NodeInfo>
)]
pub(crate) struct ApiResponse<T: Serialize> {
  pub code: i32,
  /// ok
  #[schema(example = "ok")]
  pub msg: String,
  pub data: T,
}

impl<T> ApiResponse<T>
where
  T: Serialize,
{
  fn new(code: i32, msg: String, data: T) -> Self {
    Self { code, msg, data }
  }

  pub fn ok(data: T) -> Self {
    Self::new(0, "ok".to_string(), data)
  }
}
