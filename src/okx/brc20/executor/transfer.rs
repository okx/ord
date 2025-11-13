use super::*;

impl BRC20ExecutionMessage {
  pub(super) async fn execute_transfer(
    &self,
    context: &mut TableContext<'_, '_>,
    brc20_prog_client: &HttpClient,
    chain: &Chain,
    height: u32,
    blocktime: u32,
    mut block_hash: [u8; 32],
    prog_tx_idx: u64,
  ) -> Result<BRC20Receipt, ExecutionError> {
    let BRC20Operation::Transfer { ticker, amount } = &self.operation else {
      unreachable!()
    };

    // load ticker info, ensure the ticker is deployed
    let mut ticker_info = context
      .load_brc20_ticker_info(ticker)?
      .ok_or(BRC20Error::TickerNotFound(ticker.clone().to_string()))?;

    let decimals = ticker_info.decimals;

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

    let deposited_to_brc20_prog = receiver.op_return_prog()
      && ((ticker.len() == PREDEPLOYED_TICKER_LENGTH
        && height >= HardForks::brc20_prog_activation_height(chain))
        || height >= HardForks::brc20_prog_all_tickers_activation_height(chain));

    let burned = receiver.op_return() && !deposited_to_brc20_prog;

    if burned {
      ticker_info.burned = ticker_info
        .burned
        .checked_add(*amount)
        .expect("Addition overflow");

      context.update_brc20_ticker_info(&ticker, ticker_info)?;
    }

    if deposited_to_brc20_prog {
      block_hash.reverse();

      brc20_prog_client
        .brc20_deposit(
          hex::encode(self.sender.to_script_bytes()),
          ticker.to_lowercase().to_string(),
          (*amount).into(),
          blocktime as u64,
          block_hash.into(),
          prog_tx_idx,
          self.inscription_id.to_string(),
        )
        .await
        .expect("Check your BRC2.0 server");
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
        decimals,
        send_to_coinbase,
        burned,
        deposited_to_brc20_prog,
      })),
      prog_tx_count: if deposited_to_brc20_prog { 1 } else { 0 },
    })
  }
}
