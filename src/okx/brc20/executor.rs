use super::*;
use crate::okx::brc20::entry::{BRC20Balance, BRC20Receipt, BRC20TickerInfo};
use crate::okx::brc20::error::BRC20Error;
use crate::okx::brc20::event::{
  BRC20Event, BRC20OpType, DeployEvent, InscribeTransferEvent, MintEvent, TransferEvent,
};
use bigdecimal::num_bigint::Sign;

/// Represents a message used for executing BRC20 operations.
pub(crate) struct BRC20ExecutionMessage {
  txid: Txid,
  offset: u64,
  inscription_id: InscriptionId,
  sequence_number: u32,
  inscription_number: i32,
  old_satpoint: SatPoint,
  new_satpoint: SatPoint,
  sender: UtxoAddress,
  receiver: Option<UtxoAddress>, // no address, if unbound
  message: BRC20Message,
}

impl From<&BundleMessage> for Option<BRC20ExecutionMessage> {
  fn from(value: &BundleMessage) -> Self {
    match &value.sub_message {
      Some(SubMessage::BRC20(message)) => Some(BRC20ExecutionMessage {
        txid: value.txid,
        offset: value.offset,
        inscription_id: value.inscription_id,
        sequence_number: value.sequence_number,
        inscription_number: value.inscription_number,
        old_satpoint: value.old_satpoint,
        new_satpoint: value.new_satpoint,
        sender: value.sender.clone(),
        receiver: value.receiver.clone(),
        message: message.clone(),
      }),
      _ => None,
    }
  }
}

impl BRC20ExecutionMessage {
  pub fn execute(
    self,
    context: &mut TableContext,
    height: u32,
    chain: Chain,
    blocktime: u32,
  ) -> Result<BRC20Receipt> {
    let result = match &self.message {
      BRC20Message::Deploy(..) => self.execute_deploy(context, height, chain, blocktime),
      BRC20Message::Mint { .. } => self.execute_mint(context, height),
      BRC20Message::InscribeTransfer(_) => self.execute_inscribe_transfer(context),
      BRC20Message::Transfer { .. } => self.execute_transfer(context),
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
        op_type: BRC20OpType::from(&self.message),
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

  fn execute_deploy(
    &self,
    context: &mut TableContext,
    height: u32,
    chain: Chain,
    blocktime: u32,
  ) -> Result<BRC20Receipt, ExecutionError> {
    let BRC20Message::Deploy(deploy) = &self.message else {
      unreachable!()
    };
    if self.new_satpoint.outpoint.txid != self.txid {
      return Err(ExecutionError::ExecutionFailed(
        BRC20Error::InscribeToDifferentTx,
      ));
    }

    // get the deployer address.
    let deployer = self.receiver.clone().unwrap();
    let ticker = BRC20Ticker::from_str(&deploy.tick).map_err(BRC20Error::TickerParse)?;

    // check if the ticker is not already deployed.
    if context.load_brc20_ticker_info(&ticker)?.is_some() {
      return Err(ExecutionError::ExecutionFailed(
        BRC20Error::DuplicateDeployment(ticker.to_string()),
      ));
    }

    let self_minted = deploy.self_mint.unwrap_or_default();

    let decimals = validate_and_parse_decimals(deploy.decimals.as_ref())?;

    let total_supply =
      validate_and_parse_supply(&deploy.max_supply, self_minted, decimals, height, &chain)?;

    let max_mint_limit = validate_and_parse_mint_limit(
      deploy.mint_limit.as_ref().unwrap_or(&deploy.max_supply),
      decimals,
    )?;

    let ticker_info = BRC20TickerInfo {
      ticker: ticker.clone(),
      sequence_number: self.sequence_number,
      inscription_number: self.inscription_number,
      inscription_id: self.inscription_id,
      total_supply,
      burned: 0,
      minted: 0,
      max_mint_limit,
      decimals,
      deployer: deployer.clone(),
      self_minted,
      deployed_block_height: height,
      deployed_timestamp: blocktime,
      latest_minted_block_height: height,
    };

    // insert the ticker info to the table.
    context.update_brc20_ticker_info(&ticker, ticker_info)?;

    Ok(BRC20Receipt {
      inscription_id: self.inscription_id,
      sequence_number: self.sequence_number,
      inscription_number: self.inscription_number,
      old_satpoint: self.old_satpoint,
      new_satpoint: self.new_satpoint,
      sender: self.sender.clone(),
      receiver: deployer,
      op_type: BRC20OpType::Deploy,
      result: Ok(BRC20Event::Deploy(DeployEvent {
        ticker,
        total_supply,
        decimals,
        self_minted,
        max_mint_limit,
      })),
    })
  }

  fn execute_mint(
    &self,
    context: &mut TableContext,
    height: u32,
  ) -> Result<BRC20Receipt, ExecutionError> {
    let BRC20Message::Mint { op: mint, parent } = &self.message else {
      unreachable!()
    };

    if self.new_satpoint.outpoint.txid != self.txid {
      return Err(ExecutionError::ExecutionFailed(
        BRC20Error::InscribeToDifferentTx,
      ));
    }

    let receiver = self.receiver.clone().unwrap();

    let ticker = BRC20Ticker::from_str(&mint.tick).map_err(BRC20Error::TickerParse)?;

    // load ticker info, ensure the ticker is deployed
    let mut ticker_info = context
      .load_brc20_ticker_info(&ticker)?
      .ok_or(BRC20Error::TickerNotFound(mint.tick.clone()))?;

    // check if self mint is allowed.
    if ticker_info.self_minted && !parent.is_some_and(|parent| parent == ticker_info.inscription_id)
    {
      return Err(ExecutionError::ExecutionFailed(
        BRC20Error::SelfMintPermissionDenied,
      ));
    }

    let (actual_amount, clipped) =
      validate_and_increase_minted(&mut ticker_info, &mint.amount, height)?;
    // update the ticker info.
    context.update_brc20_ticker_info(&ticker, ticker_info)?;

    let mut receiver_balance = context
      .load_brc20_balance(&receiver, &ticker)?
      .unwrap_or(BRC20Balance::new_with_ticker(&ticker));

    receiver_balance.total = Num::from(receiver_balance.total)
      .checked_add(&actual_amount)
      .map_err(BRC20Error::NumericError)?
      .checked_to_u128()
      .map_err(BRC20Error::NumericError)?;

    receiver_balance.available = Num::from(receiver_balance.available)
      .checked_add(&actual_amount)
      .map_err(BRC20Error::NumericError)?
      .checked_to_u128()
      .map_err(BRC20Error::NumericError)?;

    context.update_brc20_balance(&receiver, &ticker, receiver_balance)?;

    Ok(BRC20Receipt {
      inscription_id: self.inscription_id,
      sequence_number: self.sequence_number,
      inscription_number: self.inscription_number,
      old_satpoint: self.old_satpoint,
      new_satpoint: self.new_satpoint,
      sender: self.sender.clone(),
      receiver,
      op_type: BRC20OpType::Mint,
      result: Ok(BRC20Event::Mint(MintEvent {
        ticker,
        amount: actual_amount
          .checked_to_u128()
          .map_err(BRC20Error::NumericError)?,
        clipped,
      })),
    })
  }

  fn execute_inscribe_transfer(
    &self,
    context: &mut TableContext,
  ) -> Result<BRC20Receipt, ExecutionError> {
    let BRC20Message::InscribeTransfer(transfer) = &self.message else {
      unreachable!()
    };
    if self.new_satpoint.outpoint.txid != self.txid {
      return Err(ExecutionError::ExecutionFailed(
        BRC20Error::InscribeToDifferentTx,
      ));
    }

    let address = self.receiver.clone().unwrap();

    let ticker = BRC20Ticker::from_str(&transfer.tick).map_err(BRC20Error::TickerParse)?;

    // load ticker info, ensure the ticker is deployed
    let ticker_info = context
      .load_brc20_ticker_info(&ticker)?
      .ok_or(BRC20Error::TickerNotFound(transfer.tick.clone()))?;

    let amount = validate_inscribe_transfer_amount(&ticker_info, &transfer.amount)?;

    let mut balance = context
      .load_brc20_balance(&address, &ticker)?
      .unwrap_or(BRC20Balance::new_with_ticker(&ticker));

    balance.total = Num::from(balance.total)
      .checked_sub(&amount)
      .map_err(|_| {
        ExecutionError::ExecutionFailed(BRC20Error::InsufficientBalance(
          balance.total.to_string(),
          amount.to_string(),
        ))
      })?
      .checked_to_u128()
      .map_err(BRC20Error::NumericError)?;

    context.update_brc20_balance(&address, &ticker, balance)?;

    let amount = amount.checked_to_u128().map_err(BRC20Error::NumericError)?;

    let transferring_asset = BRC20TransferAsset {
      ticker: ticker.clone(),
      amount,
      owner: address.clone(),
      sequence_number: self.sequence_number,
      inscription_number: self.inscription_number,
      inscription_id: self.inscription_id,
    };

    context.update_brc20_transferring_asset(
      &address,
      &ticker,
      self.new_satpoint,
      transferring_asset,
    )?;

    Ok(BRC20Receipt {
      inscription_id: self.inscription_id,
      sequence_number: self.sequence_number,
      inscription_number: self.inscription_number,
      old_satpoint: self.old_satpoint,
      new_satpoint: self.new_satpoint,
      sender: self.sender.clone(),
      receiver: address,
      op_type: BRC20OpType::InscribeTransfer,
      result: Ok(BRC20Event::InscribeTransfer(InscribeTransferEvent {
        ticker,
        amount,
      })),
    })
  }
  fn execute_transfer(&self, context: &mut TableContext) -> Result<BRC20Receipt, ExecutionError> {
    let BRC20Message::Transfer { ticker, amount } = &self.message else {
      unreachable!()
    };

    // load ticker info, ensure the ticker is deployed
    let mut ticker_info = context
      .load_brc20_ticker_info(&ticker)?
      .ok_or(anyhow!("Ticker not found: {}", ticker.to_string()))?;

    // check if the sender has enough balance and update the balance
    let mut sender_balance = context
      .load_brc20_balance(&self.sender, ticker)?
      .unwrap_or(BRC20Balance::new_with_ticker(ticker));

    sender_balance.total = sender_balance.total.checked_sub(*amount).ok_or(anyhow!(
      "Subtraction overflow: sender_balance.total {} - {}",
      sender_balance.total,
      amount
    ))?;

    assert!(sender_balance.total >= sender_balance.available);

    context.update_brc20_balance(&self.sender, ticker, sender_balance)?;

    let (receiver, send_to_coinbase) = if self.new_satpoint.outpoint.txid == self.txid {
      (self.receiver.clone().unwrap(), false)
    } else {
      (self.sender.clone(), true)
    };

    // update the recipient balance
    let mut receiver_balance = context
      .load_brc20_balance(&receiver, ticker)?
      .unwrap_or(BRC20Balance::new_with_ticker(ticker));
    receiver_balance.total = receiver_balance.total.checked_add(*amount).ok_or(anyhow!(
      "Addition overflow: receiver_balance.total {} + {}",
      receiver_balance.total,
      amount
    ))?;

    receiver_balance.available = receiver_balance
      .available
      .checked_add(*amount)
      .ok_or(anyhow!(
        "Addition overflow: receiver_balance.available {} + {}",
        receiver_balance.available,
        amount
      ))?;

    context.update_brc20_balance(&receiver, ticker, receiver_balance)?;

    let burned = receiver.op_return();
    if burned {
      ticker_info.burned = ticker_info.burned.checked_add(*amount).ok_or(anyhow!(
        "Addition overflow: ticker_info.burned {} + {}",
        ticker_info.burned,
        amount
      ))?;

      context.update_brc20_ticker_info(&ticker_info.ticker.clone(), ticker_info)?;
    }
    Ok(BRC20Receipt {
      inscription_id: self.inscription_id,
      sequence_number: self.sequence_number,
      inscription_number: self.inscription_number,
      old_satpoint: self.old_satpoint,
      new_satpoint: self.new_satpoint,
      sender: self.sender.clone(),
      receiver,
      op_type: BRC20OpType::Transfer,
      result: Ok(BRC20Event::Transfer(TransferEvent {
        ticker: ticker.clone(),
        amount: *amount,
        send_to_coinbase,
        burned,
      })),
    })
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

fn validate_and_parse_decimals(
  opt_decimals: Option<&String>,
) -> std::result::Result<u8, BRC20Error> {
  let uncheck = Num::from_str(opt_decimals.unwrap_or(&MAX_DECIMAL_WIDTH.to_string()))
    .map_err(BRC20Error::NumericError)?
    .checked_to_u128()
    .map_err(BRC20Error::NumericError)?;
  if uncheck > u128::from(MAX_DECIMAL_WIDTH) {
    return Err(BRC20Error::DecimalsExceedLimit(uncheck.to_string()));
  } else {
    Ok(u8::try_from(uncheck).unwrap())
  }
}

fn validate_and_parse_supply(
  supply: &str,
  self_minted: bool,
  dec: u8,
  height: u32,
  chain: &Chain,
) -> Result<u128, BRC20Error> {
  let mut supply = supply.to_string();
  if self_minted {
    if height < HardForks::self_issuance_activation_height(chain) {
      return Err(BRC20Error::SelfIssuanceNotActivated);
    }
    if supply == u64::MIN.to_string() {
      supply = u64::MAX.to_string();
    }
  }

  let supply = Num::from_str(&supply).map_err(BRC20Error::NumericError)?;
  if supply.sign() == Sign::NoSign
    || supply > MAXIMUM_SUPPLY.to_owned()
    || supply.scale() > i64::from(dec)
  {
    return Err(BRC20Error::InvalidSupply(supply.to_string()));
  }

  let base = BIGDECIMAL_TEN
    .checked_powu(u64::from(dec))
    .map_err(BRC20Error::NumericError)?;
  supply
    .checked_mul(&base)
    .map_err(BRC20Error::NumericError)?
    .checked_to_u128()
    .map_err(BRC20Error::NumericError)
}

fn validate_and_parse_mint_limit(limit: &str, dec: u8) -> Result<u128, BRC20Error> {
  let limit = Num::from_str(limit).map_err(BRC20Error::NumericError)?;

  if limit.sign() == Sign::NoSign
    || limit > MAXIMUM_SUPPLY.to_owned()
    || limit.scale() > i64::from(dec)
  {
    return Err(BRC20Error::InvalidMaxMintLimit(limit.to_string()));
  }

  let base = BIGDECIMAL_TEN
    .checked_powu(u64::from(dec))
    .map_err(BRC20Error::NumericError)?;
  limit
    .checked_mul(&base)
    .map_err(BRC20Error::NumericError)?
    .checked_to_u128()
    .map_err(BRC20Error::NumericError)
}

fn validate_and_increase_minted(
  ticker_info: &mut BRC20TickerInfo,
  amount: &str,
  height: u32,
) -> std::result::Result<(Num, bool), BRC20Error> {
  let amount = Num::from_str(amount).map_err(BRC20Error::NumericError)?;

  if amount.scale() > i64::from(ticker_info.decimals) {
    return Err(BRC20Error::DecimalsExceedLimit(amount.to_string()));
  }

  let base = BIGDECIMAL_TEN
    .checked_powu(u64::from(ticker_info.decimals))
    .map_err(BRC20Error::NumericError)?;

  let amount = amount
    .checked_mul(&base)
    .map_err(BRC20Error::NumericError)?;

  // cannot mint zero amount.
  if amount.sign() == Sign::NoSign {
    return Err(BRC20Error::InvalidAmount(amount.to_string()));
  }

  if amount > Num::from(ticker_info.max_mint_limit) {
    return Err(BRC20Error::MintAmountExceedLimit(amount.to_string()));
  }

  let minted = Num::from(ticker_info.minted);
  let supply = Num::from(ticker_info.total_supply);

  if minted >= supply {
    return Err(BRC20Error::MintingLimitReached);
  }

  // cut off any excess.
  let (amount, clipped) = if amount
    .checked_add(&minted)
    .map_err(BRC20Error::NumericError)?
    > supply
  {
    (
      supply
        .checked_sub(&minted)
        .map_err(BRC20Error::NumericError)?,
      true,
    )
  } else {
    (amount, false)
  };

  ticker_info.minted = minted
    .checked_add(&amount)
    .map_err(BRC20Error::NumericError)?
    .checked_to_u128()
    .map_err(BRC20Error::NumericError)?;
  ticker_info.latest_minted_block_height = height;
  Ok((amount, clipped))
}

fn validate_inscribe_transfer_amount(
  ticker_info: &BRC20TickerInfo,
  amount: &str,
) -> std::result::Result<Num, BRC20Error> {
  let amount = Num::from_str(amount).map_err(BRC20Error::NumericError)?;
  if amount.scale() > i64::from(ticker_info.decimals) {
    return Err(BRC20Error::DecimalsExceedLimit(amount.to_string()));
  }

  let base = BIGDECIMAL_TEN
    .checked_powu(u64::from(ticker_info.decimals))
    .map_err(BRC20Error::NumericError)?;

  let amount = amount
    .checked_mul(&base)
    .map_err(BRC20Error::NumericError)?;

  if amount.sign() == Sign::NoSign {
    return Err(BRC20Error::InvalidAmount(amount.to_string()));
  }
  Ok(amount)
}
