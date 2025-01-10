use super::*;

impl BRC20ExecutionMessage {
  pub(super) fn execute_inscribe_transfer(
    &self,
    context: &mut TableContext,
  ) -> Result<BRC20Receipt, ExecutionError> {
    let BRC20Operation::InscribeTransfer(transfer) = &self.operation else {
      unreachable!()
    };

    let ticker = BRC20Ticker::from_str(&transfer.tick).map_err(BRC20Error::TickerParse)?;

    // load ticker info, ensure the ticker is deployed
    let ticker_info = context
      .load_brc20_ticker_info(&ticker)?
      .ok_or(BRC20Error::TickerNotFound(transfer.tick.clone()))?;

    let ticker = ticker_info.ticker;

    let amt = FixedPoint::new_from_str(&transfer.amount, ticker_info.decimals)
      .map_err(BRC20Error::NumericError)?;
    if amt.is_zero()
      || amt > FixedPoint::new_unchecked(ticker_info.total_supply, ticker_info.decimals)
    {
      return Err(ExecutionError::ExecutionFailed(BRC20Error::InvalidAmount(
        amt.to_string(),
      )));
    }

    let address = self.receiver.clone().unwrap();

    let mut balance = context
      .load_brc20_balance(&address, &ticker)?
      .unwrap_or(BRC20Balance::new_with_ticker(&ticker));

    balance.available = FixedPoint::new_unchecked(balance.available, ticker_info.decimals)
      .checked_sub(amt)
      .ok_or(ExecutionError::ExecutionFailed(
        BRC20Error::InsufficientBalance(balance.total.to_string(), amt.to_string()),
      ))?
      .to_u128_and_scale()
      .0;

    context.update_brc20_balance(&address, &ticker, balance)?;

    let transferring_asset = BRC20TransferAsset {
      ticker: ticker.clone(),
      amount: amt.to_u128_and_scale().0,
      owner: address.clone(),
      sequence_number: self.sequence_number,
      inscription_number: self.inscription_number,
      inscription_id: self.inscription_id,
    };

    context.insert_brc20_transferring_asset(
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
        amount: amt.to_u128_and_scale().0,
      })),
    })
  }
}
