use crate::okx::datastore::brc20::Receipt;
use crate::okx::datastore::ord::InscriptionOp;
use serde::{Deserialize, Serialize};

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct ExecuteTxResponse {
  pub brc20_receipts: Vec<Receipt>,
  pub ord_operations: Vec<InscriptionOp>,
}
