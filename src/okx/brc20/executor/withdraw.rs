use {super::*, crate::okx::utxo_address::BRC20_PROG_OP_RETURN_UTXO_ADDRESS};

impl BRC20ExecutionMessage {
  pub(super) fn execute_withdraw(
    &self,
    context: &mut TableContext<'_, '_>,
    brc20_prog_client: &Brc20ProgClient,
    block_timestamp: u64,
    block_hash: &BlockHash,
    tx_idx: u64,
  ) -> Result<BRC20Receipt, ExecutionError> {
    let BRC20Operation::Withdraw { ticker, amount } = &self.operation else {
      unreachable!()
    };

    // load ticker info, ensure the ticker is deployed
    let ticker_info = context
      .load_brc20_ticker_info(ticker)?
      .ok_or(BRC20Error::TickerNotFound(ticker.clone().to_string()))?;

    let decimals = ticker_info.decimals;

    let receipt = brc20_prog_client
      .brc20_withdraw(
        hex::encode(self.sender.to_script_bytes()),
        ticker.to_lowercase().to_string(),
        (*amount).into(),
        block_timestamp,
        block_hash.to_evm_hash(),
        tx_idx,
        self.inscription_id.to_string(),
      )
      .expect("Check your BRC2.0 server");

    let success = !receipt.status.is_zero();

    if success {
      let mut prog_balance = context
        .load_brc20_balance(&BRC20_PROG_OP_RETURN_UTXO_ADDRESS, &ticker)?
        .unwrap_or(BRC20Balance::new_with_ticker(&ticker));

      prog_balance.total = prog_balance
        .total
        .checked_sub(*amount)
        .expect("Subtraction overflow");

      context.update_brc20_balance(&BRC20_PROG_OP_RETURN_UTXO_ADDRESS, &ticker, prog_balance)?;

      let receiver = if self.new_satpoint.outpoint.txid == self.txid {
        self.receiver.clone().unwrap()
      } else {
        self.sender.clone()
      };

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
    }

    Ok(BRC20Receipt {
      inscription_id: self.inscription_id.clone(),
      sequence_number: self.sequence_number,
      inscription_number: self.inscription_number,
      old_satpoint: self.old_satpoint,
      new_satpoint: self.new_satpoint,
      op_type: BRC20OpType::Withdraw,
      sender: self.sender.clone(),
      receiver: self.receiver.clone().unwrap_or(self.sender.clone()),
      result: Ok(BRC20Event::Withdraw(WithdrawEvent {
        ticker: ticker.clone(),
        amount: *amount,
        decimals,
        valid: success,
      })),
      prog_tx_count: 1,
    })
  }
}
