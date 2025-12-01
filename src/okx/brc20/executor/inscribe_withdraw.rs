use super::*;

impl BRC20ExecutionMessage {
  pub(super) fn execute_inscribe_withdraw(
    &self,
    context: &mut TableContext,
  ) -> Result<BRC20Receipt, ExecutionError> {
    let BRC20Operation::InscribeWithdraw(inscribe_withdraw) = &self.operation else {
      unreachable!()
    };

    // load ticker info, ensure the ticker is deployed
    let ticker = BRC20Ticker::from_str(&inscribe_withdraw.tick).map_err(BRC20Error::TickerParse)?;

    let ticker_info = context
      .load_brc20_ticker_info(&ticker)?
      .ok_or(BRC20Error::TickerNotFound(ticker.clone().to_string()))?;

    let amt = FixedPoint::new_from_str(&inscribe_withdraw.amount, ticker_info.decimals)
      .map_err(BRC20Error::NumericError)?;
    if amt.is_zero()
      || amt > FixedPoint::new_unchecked(ticker_info.total_supply, ticker_info.decimals)
    {
      return Err(ExecutionError::ExecutionFailed(BRC20Error::InvalidAmount(
        amt,
      )));
    }

    context.insert_brc20_withdraw_asset(
      self.new_satpoint,
      entry::BRC20Withdraw {
        ticker: ticker.clone(),
        amount: amt.to_u128_and_scale().0,
        owner: self.receiver.clone().unwrap(),
        sequence_number: self.sequence_number,
        inscription_number: self.inscription_number,
        inscription_id: self.inscription_id.clone(),
      },
    )?;

    let event = BRC20Event::InscribeWithdraw(InscribeWithdrawEvent {
      ticker: ticker.clone(),
      amount: amt.to_u128_and_scale().0,
      decimals: ticker_info.decimals,
    });

    Ok(BRC20Receipt {
      inscription_id: self.inscription_id.clone(),
      sequence_number: self.sequence_number,
      inscription_number: self.inscription_number,
      old_satpoint: self.old_satpoint,
      new_satpoint: self.new_satpoint,
      op_type: BRC20OpType::InscribeWithdraw,
      sender: self.sender.clone(),
      receiver: self.receiver.clone().unwrap_or(self.sender.clone()),
      result: Ok(event),
    })
  }
}
