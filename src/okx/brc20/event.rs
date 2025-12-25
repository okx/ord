use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum BRC20OpType {
  Predeploy,
  Deploy,
  Mint,
  InscribeTransfer,
  Transfer,
  InscribeProgDeploy,
  ProgDeploy,
  InscribeProgCall,
  ProgCall,
  InscribeProgTransact,
  ProgTransact,
  InscribeWithdraw,
  Withdraw,
}

impl From<&BRC20Operation> for BRC20OpType {
  fn from(value: &BRC20Operation) -> Self {
    match value {
      BRC20Operation::Predeploy(_) => BRC20OpType::Predeploy,
      BRC20Operation::Deploy { .. } => BRC20OpType::Deploy,
      BRC20Operation::Mint { .. } => BRC20OpType::Mint,
      BRC20Operation::InscribeTransfer(_) => BRC20OpType::InscribeTransfer,
      BRC20Operation::Transfer { .. } => BRC20OpType::Transfer,
      BRC20Operation::InscribeProgDeploy { .. } => BRC20OpType::InscribeProgDeploy,
      BRC20Operation::ProgDeploy { .. } => BRC20OpType::ProgDeploy,
      BRC20Operation::InscribeProgCall { .. } => BRC20OpType::InscribeProgCall,
      BRC20Operation::ProgCall { .. } => BRC20OpType::ProgCall,
      BRC20Operation::InscribeProgTransact { .. } => BRC20OpType::InscribeProgTransact,
      BRC20Operation::ProgTransact { .. } => BRC20OpType::ProgTransact,
      BRC20Operation::InscribeWithdraw(_) => BRC20OpType::InscribeWithdraw,
      BRC20Operation::Withdraw { .. } => BRC20OpType::Withdraw,
    }
  }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum BRC20Event {
  Predeploy(PredeployEvent),
  Deploy(DeployEvent),
  Mint(MintEvent),
  InscribeTransfer(InscribeTransferEvent),
  Transfer(TransferEvent),
  InscribeProgDeploy(InscribeProgDeployEvent),
  ProgDeploy(ProgDeployEvent),
  InscribeProgCall(InscribeProgCallEvent),
  ProgCall(ProgCallEvent),
  InscribeProgTransact(InscribeProgTransactEvent),
  ProgTransact(ProgTransactEvent),
  InscribeWithdraw(InscribeWithdrawEvent),
  Withdraw(WithdrawEvent),
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct PredeployEvent {
  pub hash: String,
  pub predeployer: UtxoAddress,
  pub block_height: u32,
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
  pub original_ticker: BRC20Ticker,
  pub ticker: BRC20Ticker,
  pub amount: u128,
  pub decimals: u8,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct MintEvent {
  pub original_ticker: BRC20Ticker,
  pub ticker: BRC20Ticker,
  pub amount: u128,
  pub decimals: u8,
  pub clipped: bool,
  pub parent_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct TransferEvent {
  pub original_ticker: BRC20Ticker,
  pub ticker: BRC20Ticker,
  pub amount: u128,
  pub decimals: u8,
  pub send_to_coinbase: bool,
  pub burned: bool,
  pub deposited_to_brc20_prog: bool,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
pub struct InscribeProgDeployEvent {
  pub data: Option<String>,
  pub base64_data: Option<String>,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
pub struct ProgDeployEvent {
  pub data: Option<String>,
  pub base64_data: Option<String>,
  pub inscription_byte_length: u32,
  pub op_return_tx_id: Option<String>,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
pub struct InscribeProgCallEvent {
  pub contract_address: Option<String>,
  pub inscription_id: Option<String>,
  pub data: Option<String>,
  pub base64_data: Option<String>,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
pub struct ProgCallEvent {
  pub contract_address: Option<String>,
  pub inscription_id: Option<String>,
  pub data: Option<String>,
  pub base64_data: Option<String>,
  pub inscription_byte_length: u32,
  pub op_return_tx_id: Option<String>,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
pub struct InscribeProgTransactEvent {
  pub data: Option<String>,
  pub base64_data: Option<String>,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
pub struct ProgTransactEvent {
  pub data: Option<String>,
  pub base64_data: Option<String>,
  pub inscription_byte_length: u32,
  pub op_return_tx_id: Option<String>,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
pub struct InscribeWithdrawEvent {
  pub original_ticker: BRC20Ticker,
  pub ticker: BRC20Ticker,
  pub amount: u128,
  pub decimals: u8,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
pub struct WithdrawEvent {
  pub original_ticker: BRC20Ticker,
  pub ticker: BRC20Ticker,
  pub amount: u128,
  pub decimals: u8,
}
