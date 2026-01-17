use {
  super::*,
  crate::{
    okx::{brc20::HardForks, UtxoAddress},
    Chain, InscriptionId,
  },
  bitcoin::Txid,
};

/// Executor for processing swap module balance refunds at a specific block height.
/// This handles the hardcoded refund logic that executes before normal block processing.
pub(crate) struct SwapModuleRefundExecutor {
  txid: Txid,
  refund_from: UtxoAddress,
  refund_to: UtxoAddress,
}

impl SwapModuleRefundExecutor {
  /// Creates a refund executor if the current block height matches the refund activation height.
  pub fn from_block(height: u32, block: &BlockData, chain: &Chain) -> Option<Self> {
    if height != HardForks::brc20_swap_refund_activation_height(chain) {
      return None;
    }
    let (refund_from, refund_to) = HardForks::brc20_swap_refund_addresses(chain)?;

    Some(Self {
      txid: block.txdata.first().map(|(_, txid)| *txid)?,
      refund_from,
      refund_to,
    })
  }

  /// Executes the refund process for all balances in the sender address.
  pub fn execute(self, context: &mut TableContext<'_, '_>) -> Result<(Txid, Vec<BRC20Receipt>)> {
    let mut receipts = Vec::new();
    let balances = context.load_brc20_balances_by_address(&self.refund_from)?;

    for balance in balances {
      if balance.total <= 0 {
        continue;
      }

      // Process refund for this ticker
      receipts.append(&mut process_ticker_refund(
        context,
        &balance.ticker,
        &self.refund_from,
        &self.refund_to,
      )?);
    }
    Ok((self.txid, receipts))
  }
}

/// Processes the refund for a specific ticker by transferring the entire balance
/// from the sender address to the receiver address.
pub(crate) fn process_ticker_refund(
  context: &mut TableContext<'_, '_>,
  ticker: &BRC20Ticker,
  from: &UtxoAddress,
  to: &UtxoAddress,
) -> Result<Vec<BRC20Receipt>> {
  let ticker_info = context
    .load_brc20_ticker_info(ticker)?
    .ok_or(BRC20Error::TickerNotFound(ticker.clone().to_string()))?;

  let mut from_balance = context
    .load_brc20_balance(from, ticker)?
    .unwrap_or(BRC20Balance::new_with_ticker(ticker));

  let refund_amount = from_balance.total;

  from_balance.total = 0;
  from_balance.available = 0;
  context.update_brc20_balance(from, ticker, from_balance)?;

  // Update receiver balance
  let mut to_balance = context
    .load_brc20_balance(to, ticker)?
    .unwrap_or(BRC20Balance::new_with_ticker(ticker));

  to_balance.total = to_balance
    .total
    .checked_add(refund_amount)
    .expect("Balance overflow during refund");

  to_balance.available = to_balance
    .available
    .checked_add(refund_amount)
    .expect("Balance overflow during refund");

  context.update_brc20_balance(to, ticker, to_balance)?;

  let inscription_id = generate_refund_inscription_id(&ticker.to_lowercase());
  Ok(vec![
    BRC20Receipt {
      inscription_id: inscription_id.clone(),
      sequence_number: u32::MAX,
      inscription_number: i32::MAX,
      old_satpoint: Default::default(),
      new_satpoint: Default::default(),
      sender: from.clone(),
      receiver: from.clone(),
      op_type: BRC20OpType::InscribeTransfer,
      result: Ok(BRC20Event::InscribeTransfer(InscribeTransferEvent {
        original_ticker: ticker_info.ticker.clone(),
        ticker: ticker_info.ticker.clone(),
        amount: refund_amount,
        decimals: ticker_info.decimals,
      })),
    },
    BRC20Receipt {
      inscription_id,
      sequence_number: u32::MAX,
      inscription_number: i32::MAX,
      old_satpoint: Default::default(),
      new_satpoint: Default::default(),
      sender: from.clone(),
      receiver: to.clone(),
      op_type: BRC20OpType::Transfer,
      result: Ok(BRC20Event::Transfer(TransferEvent {
        original_ticker: ticker_info.ticker.clone(),
        ticker: ticker_info.ticker.clone(),
        amount: refund_amount,
        decimals: ticker_info.decimals,
        send_to_coinbase: false,
        burned: false,
        deposited_to_brc20_prog: false,
      })),
    },
  ])
}

/// Generates a synthetic inscription ID for swap refund operations.
/// The ID is derived from the ticker name and a constant prefix.
fn generate_refund_inscription_id(ticker: &BRC20LowerCaseTicker) -> InscriptionId {
  let mut id_hex = String::with_capacity(66);
  id_hex.push_str(&hex::encode("BRC20SWAPREFUND"));
  id_hex.push_str(&hex::encode(ticker.to_string().as_bytes()));

  let missing = 64_usize.saturating_sub(id_hex.len());
  if missing > 0 {
    id_hex.extend(std::iter::repeat('0').take(missing));
  } else if id_hex.len() > 64 {
    id_hex.truncate(64);
  }

  id_hex.push_str("i0");
  InscriptionId::from_str(&id_hex).unwrap()
}
