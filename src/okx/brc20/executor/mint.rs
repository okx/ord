use super::*;

impl BRC20ExecutionMessage {
  pub(super) fn execute_mint(
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

    // check if minting limit is reached.
    if ticker_info.minted >= ticker_info.total_supply {
      return Err(ExecutionError::ExecutionFailed(
        BRC20Error::MintingLimitReached,
      ));
    }

    let mut amt = FixedPoint::new_from_str(&mint.amount, ticker_info.decimals)
      .map_err(BRC20Error::NumericError)?;

    // can not mint zero amount.
    if amt.is_zero() {
      return Err(ExecutionError::ExecutionFailed(BRC20Error::InvalidAmount(
        amt.to_string(),
      )));
    }

    // check if the mint amount exceeds the allowed limit.
    if amt > FixedPoint::new_unchecked(ticker_info.max_mint_limit, ticker_info.decimals) {
      return Err(ExecutionError::ExecutionFailed(
        BRC20Error::MintAmountExceedLimit(amt.to_string()),
      ));
    }

    // cut off any excess.
    let minted = FixedPoint::new_unchecked(ticker_info.minted, ticker_info.decimals);
    let supply = FixedPoint::new_unchecked(ticker_info.total_supply, ticker_info.decimals);
    let mut clipped = false;
    if amt + minted > supply {
      clipped = true;
      amt = supply - minted
    }

    // get user's balances
    let receiver = self.receiver.clone().unwrap();
    let mut receiver_balance = context
      .load_brc20_balance(&receiver, &ticker)?
      .unwrap_or(BRC20Balance::new_with_ticker(&ticker));

    receiver_balance.total =
      (FixedPoint::new_unchecked(receiver_balance.total, ticker_info.decimals) + amt)
        .to_u128_and_scale()
        .0;
    receiver_balance.available =
      (FixedPoint::new_unchecked(receiver_balance.available, ticker_info.decimals) + amt)
        .to_u128_and_scale()
        .0;
    context.update_brc20_balance(&receiver, &ticker, receiver_balance)?;

    // update the ticker info.
    ticker_info.minted = (minted + amt).to_u128_and_scale().0;
    ticker_info.latest_minted_block_height = height;
    context.update_brc20_ticker_info(&ticker, ticker_info)?;

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
        amount: amt.to_u128_and_scale().0,
        clipped,
      })),
    })
  }
}
