use {super::*, utoipa::ToSchema};

#[derive(Default, Debug, Clone, Serialize, Deserialize, ToSchema)]
#[schema(as = brc20::TransferableInscription)]
#[serde(rename_all = "camelCase")]
pub struct TransferableInscription {
  /// The inscription id.
  pub inscription_id: String,
  /// The inscription number.
  pub inscription_number: i32,
  /// The amount of the ticker that will be transferred.
  #[schema(format = "uint64")]
  pub amount: String,
  /// The ticker name that will be transferred.
  pub tick: String,
  /// The address to which the transfer will be made.
  pub owner: String,

  pub satpoint: SatPoint,
}
