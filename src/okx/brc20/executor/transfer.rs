use super::*;

impl BRC20ExecutionMessage {
  pub(super) fn execute_transfer(
    &self,
    context: &mut TableContext,
  ) -> Result<BRC20Receipt, ExecutionError> {
    let BRC20Operation::Transfer { ticker, amount } = &self.operation else {
      unreachable!()
    };

    // load ticker info, ensure the ticker is deployed
    let mut ticker_info = context
      .load_brc20_ticker_info(ticker)?
      .ok_or(BRC20Error::TickerNotFound(ticker.clone().to_string()))?;

    let ticker = ticker_info.ticker.clone();

    // check if the sender has enough balance and update the balance
    let mut sender_balance = context
      .load_brc20_balance(&self.sender, &ticker)?
      .unwrap_or(BRC20Balance::new_with_ticker(&ticker));

    sender_balance.total = sender_balance
      .total
      .checked_sub(*amount)
      .expect("Subtraction overflow");

    assert!(sender_balance.total >= sender_balance.available,);

    context.update_brc20_balance(&self.sender, &ticker, sender_balance)?;

    let (receiver, send_to_coinbase) = if self.new_satpoint.outpoint.txid == self.txid {
      (self.receiver.clone().unwrap(), false)
    } else {
      (self.sender.clone(), true)
    };

    // update the recipient balance
    let mut receiver_balance = context
      .load_brc20_balance(&receiver, &ticker)?
      .unwrap_or(BRC20Balance::new_with_ticker(&ticker));

    receiver_balance.total = receiver_balance
      .total
      .checked_add(*amount)
      .expect("Addition overflow");

    receiver_balance.available = receiver_balance
      .available
      .checked_add(*amount)
      .expect("Addition overflow");

    context.update_brc20_balance(&receiver, &ticker, receiver_balance)?;

    let burned = receiver.op_return();
    if burned {
      ticker_info.burned = ticker_info
        .burned
        .checked_add(*amount)
        .expect("Addition overflow");

      context.update_brc20_ticker_info(&ticker, ticker_info)?;
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
        ticker,
        amount: *amount,
        send_to_coinbase,
        burned,
      })),
    })
  }
}
