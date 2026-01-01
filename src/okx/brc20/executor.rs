use super::{
  entry::{BRC20Balance, BRC20Predeploy, BRC20Receipt, BRC20TickerInfo},
  error::BRC20Error,
  event::{
    BRC20Event, BRC20OpType, DeployEvent, InscribeProgCallEvent, InscribeProgDeployEvent,
    InscribeProgTransactEvent, InscribeTransferEvent, InscribeWithdrawEvent, MintEvent,
    PredeployEvent, ProgCallEvent, ProgDeployEvent, ProgTransactEvent, TransferEvent,
    WithdrawEvent,
  },
  evm_prog_client::Brc20ProgClient,
  *,
};

mod deploy;
mod inscribe_prog_call;
mod inscribe_prog_deploy;
mod inscribe_prog_transact;
mod inscribe_transfer;
mod inscribe_withdraw;
mod mint;
mod predeploy;
mod prog_call;
mod prog_deploy;
mod prog_transact;
mod transfer;
mod withdraw;

/// Represents a message used for executing BRC20 operations.
pub(crate) struct BRC20ExecutionMessage {
  txid: Txid,
  inscription_id: InscriptionId,
  sequence_number: u32,
  inscription_number: i32,
  old_satpoint: SatPoint,
  new_satpoint: SatPoint,
  sender: UtxoAddress,
  receiver: Option<UtxoAddress>, // no address, if unbound
  operation: BRC20Operation,
}

impl BRC20ExecutionMessage {
  pub(crate) fn new_from_bundle_message(
    value: &BundleMessage,
    context: &mut TableContext,
  ) -> Result<Option<Self>> {
    let build_message = |operation| {
      Ok(Some(Self {
        txid: value.txid,
        inscription_id: value.inscription_id,
        sequence_number: value.sequence_number,
        inscription_number: value.inscription_number,
        old_satpoint: value.old_satpoint,
        new_satpoint: value.new_satpoint,
        sender: value.sender.clone(),
        receiver: value.receiver.clone(),
        operation,
      }))
    };

    match &value.inscription_action {
      InscriptionAction::Created { sub_type, .. } => {
        if let Some(SubType::BRC20(brc20_operation)) = sub_type {
          build_message(brc20_operation.clone())
        } else {
          Ok(None)
        }
      }
      InscriptionAction::Transferred => match Option::<TransferredInscription>::from(value) {
        Some(transferred_inscription) => {
          match transferred_inscription.extract_and_validate_transfer(context) {
            Ok(Some(brc20_operation)) => build_message(brc20_operation),
            Ok(None) => Ok(None),
            Err(err) => Err(err),
          }
        }
        _ => unreachable!(),
      },
    }
  }
}

impl BRC20ExecutionMessage {
  pub fn execute(
    self,
    context: &mut TableContext<'_, '_>,
    brc20_prog_client: &Brc20ProgClient,
    chain: &Chain,
    height: u32,
    blocktime: u32,
    block_hash: &BlockHash,
    prog_tx_idx: &mut u64,
  ) -> Result<BRC20Receipt> {
    let result = match &self.operation {
      // Core BRC20 operations
      BRC20Operation::Predeploy(_) => self.execute_predeploy(context, height),
      BRC20Operation::Deploy { .. } => self.execute_deploy(context, height, blocktime),
      BRC20Operation::Mint { .. } => self.execute_mint(context, height),
      BRC20Operation::InscribeTransfer(_) => self.execute_inscribe_transfer(context),
      BRC20Operation::Transfer { .. } => self.execute_transfer(
        context,
        brc20_prog_client,
        chain,
        height,
        blocktime,
        block_hash,
        prog_tx_idx,
      ),

      // BRC2.0 programmable module operations
      BRC20Operation::InscribeProgDeploy { .. } => self.execute_inscribe_prog_deploy(context),
      BRC20Operation::ProgDeploy { .. } => self.execute_prog_deploy(
        brc20_prog_client,
        blocktime as u64,
        block_hash,
        prog_tx_idx,
        height >= HardForks::brc20_prog_prague_activation_height(chain),
      ),
      BRC20Operation::InscribeProgCall { .. } => self.execute_inscribe_prog_call(context),
      BRC20Operation::ProgCall { .. } => self.execute_prog_call(
        brc20_prog_client,
        blocktime as u64,
        block_hash,
        prog_tx_idx,
        height >= HardForks::brc20_prog_prague_activation_height(chain),
      ),
      BRC20Operation::InscribeProgTransact { .. } => self.execute_inscribe_prog_transact(context),
      BRC20Operation::ProgTransact { .. } => self.execute_prog_transact(
        brc20_prog_client,
        blocktime as u64,
        block_hash,
        prog_tx_idx,
        height >= HardForks::brc20_prog_prague_activation_height(chain),
      ),

      // Module withdrawal operations
      BRC20Operation::InscribeWithdraw(_) => self.execute_inscribe_withdraw(context),
      BRC20Operation::Withdraw { .. } => self.execute_withdraw(
        context,
        brc20_prog_client,
        blocktime as u64,
        block_hash,
        prog_tx_idx,
      ),
    };

    match result {
      Ok(receipt) => Ok(receipt),
      Err(ExecutionError::ExecutionFailed(e)) => Ok(BRC20Receipt {
        // Handle specific execution failure
        inscription_id: self.inscription_id,
        sequence_number: self.sequence_number,
        inscription_number: self.inscription_number,
        old_satpoint: self.old_satpoint,
        new_satpoint: self.new_satpoint,
        op_type: BRC20OpType::from(&self.operation),
        sender: self.sender.clone(),
        receiver: self.receiver.unwrap_or(self.sender),
        result: Err(e),
      }),
      Err(e) => {
        log::error!(
          "BRC20 execution failed: txid = {}, inscription_id = {}, error = {:?}",
          self.txid,
          self.inscription_id,
          e
        );
        Err(e.into())
      }
    }
  }
}

#[derive(Debug, thiserror::Error)]
pub(super) enum ExecutionError {
  #[error("Storage error: {0}")]
  Storage(#[from] redb::StorageError),
  #[error("Execution failed: {0}")]
  ExecutionFailed(#[from] BRC20Error),
  #[error("Unexpected error: {0}")]
  Unexpected(#[from] Error),
}

pub(crate) fn swap_refund_by_ticker(
  context: &mut TableContext<'_, '_>,
  original_ticker: BRC20Ticker,
  sender_address: &UtxoAddress,
  receiver_address: &UtxoAddress,
) -> Result<BRC20Receipt> {
  // load ticker info, ensure the ticker is deployed
  let ticker_info =
    context
      .load_brc20_ticker_info(&original_ticker)?
      .ok_or(BRC20Error::TickerNotFound(
        original_ticker.clone().to_string(),
      ))?;

  let ticker = ticker_info.ticker.clone();
  let decimals = ticker_info.decimals;

  // check if the sender has enough balance and update the balance
  let mut sender_balance = context
    .load_brc20_balance(sender_address, &ticker)?
    .unwrap_or(BRC20Balance::new_with_ticker(&ticker));

  let amount = sender_balance.total;

  sender_balance.total = 0;
  sender_balance.available = 0;
  context.update_brc20_balance(sender_address, &ticker, sender_balance)?;

  // update the recipient balance
  let mut receiver_balance = context
    .load_brc20_balance(receiver_address, &ticker)?
    .unwrap_or(BRC20Balance::new_with_ticker(&ticker));

  receiver_balance.total = receiver_balance
    .total
    .checked_add(amount)
    .expect("Addition overflow");

  receiver_balance.available = receiver_balance
    .available
    .checked_add(amount)
    .expect("Addition overflow");

  context.update_brc20_balance(receiver_address, &ticker, receiver_balance)?;

  let (send_to_coinbase, burned, deposited_to_brc20_prog) = (false, false, false);
  Ok(BRC20Receipt {
    inscription_id: Default::default(),
    sequence_number: 0,
    inscription_number: 0,
    old_satpoint: Default::default(),
    new_satpoint: Default::default(),
    sender: sender_address.clone(),
    receiver: receiver_address.clone(),
    op_type: BRC20OpType::Transfer,
    result: Ok(BRC20Event::Transfer(TransferEvent {
      original_ticker: original_ticker.clone(),
      ticker: ticker.clone(),
      amount,
      decimals,
      send_to_coinbase,
      burned,
      deposited_to_brc20_prog,
    })),
  })
}
