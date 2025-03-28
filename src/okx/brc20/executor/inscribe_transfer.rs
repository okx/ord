use super::*;

impl BRC20ExecutionMessage {
  pub(super) fn execute_inscribe_transfer(
    &self,
    context: &mut TableContext,
  ) -> Result<BRC20Receipt, ExecutionError> {
    let BRC20Operation::InscribeTransfer{ signer, transfer } = &self.operation else {
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
        amt,
      )));
    }

    let mut sender_or_legacy = self.sender.clone();
    let receiver = self.receiver.clone().unwrap();
    let mut sender = receiver.clone();
    let mut sender_balance = context
      .load_brc20_balance(&sender, &ticker)?
      .unwrap_or(BRC20Balance::new_with_ticker(&ticker));

    let single_step_transfer = signer.is_some();
    if sender_balance.single_step_transfer && !single_step_transfer {
      return Err(ExecutionError::ExecutionFailed(
        BRC20Error::LegacyTransferPermissionDenied,
      ));
    }

    let mut receiver_balance: Option<BRC20Balance> = None;
    if let Some(signer) = signer.clone() {
      sender_or_legacy = signer.clone();
      if signer != sender {
        receiver_balance = Some(sender_balance);

        sender = signer;
        sender_balance = context
          .load_brc20_balance(&sender, &ticker)?
          .unwrap_or(BRC20Balance::new_with_ticker(&ticker));
      }
    }

    let available = FixedPoint::new_unchecked(sender_balance.available, ticker_info.decimals);
    sender_balance.available = available
      .checked_sub(amt)
      .ok_or(ExecutionError::ExecutionFailed(
        BRC20Error::InsufficientBalance(available, amt),
      ))?
      .to_u128_and_scale()
      .0;

    let amount = amt.to_u128_and_scale().0;
    if let Some(mut receiver_balance) = receiver_balance {
      sender_balance.total = sender_balance
        .total
        .checked_sub(amount)
        .expect("Subtraction overflow");

      receiver_balance.total = receiver_balance
        .total
        .checked_add(amount)
        .expect("Addition overflow");

      context.update_brc20_balance(&receiver, &ticker, receiver_balance)?;
    }

    if single_step_transfer {
      sender_balance.single_step_transfer = true;
    }
    context.update_brc20_balance(&sender, &ticker, sender_balance)?;

    let transferring_asset = BRC20TransferAsset {
      ticker: ticker.clone(),
      amount: amount,
      owner: receiver.clone(),
      sequence_number: self.sequence_number,
      inscription_number: self.inscription_number,
      inscription_id: self.inscription_id,
    };

    context.insert_brc20_transferring_asset(
      &receiver,
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
      sender: sender_or_legacy,
      receiver: receiver,
      op_type: BRC20OpType::InscribeTransfer,
      result: Ok(BRC20Event::InscribeTransfer(InscribeTransferEvent {
        ticker,
        amount: amount,
      })),
    })
  }
}
