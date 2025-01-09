use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum BRC20OpType {
  Deploy,
  Mint,
  InscribeTransfer,
  Transfer,
}

impl From<&BRC20Operation> for BRC20OpType {
  fn from(value: &BRC20Operation) -> Self {
    match value {
      BRC20Operation::Deploy(_) => BRC20OpType::Deploy,
      BRC20Operation::Mint { .. } => BRC20OpType::Mint,
      BRC20Operation::InscribeTransfer(_) => BRC20OpType::InscribeTransfer,
      BRC20Operation::Transfer { .. } => BRC20OpType::Transfer,
    }
  }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum BRC20Event {
  Deploy(DeployEvent),
  Mint(MintEvent),
  InscribeTransfer(InscribeTransferEvent),
  Transfer(TransferEvent),
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct DeployEvent {
  pub ticker: BRC20Ticker,
  pub total_supply: u128,
  pub decimals: u8,
  pub self_minted: bool,
  pub max_mint_limit: u128,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct InscribeTransferEvent {
  pub ticker: BRC20Ticker,
  pub amount: u128,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct MintEvent {
  pub ticker: BRC20Ticker,
  pub amount: u128,
  pub clipped: bool,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct TransferEvent {
  pub ticker: BRC20Ticker,
  pub amount: u128,
  pub send_to_coinbase: bool,
  pub burned: bool,
}
