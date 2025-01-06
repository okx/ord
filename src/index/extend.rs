use super::{rtx::Rtx, *};
use crate::okx::{
  brc20::{BRC20Balance, BRC20Receipt, BRC20Ticker, BRC20TickerInfo, BRC20TransferAsset},
  entry::{DynamicEntry, InscriptionReceipt},
  AddressEndpoint, AddressTickerKey, UtxoAddress,
};

mod brc20;
mod inscription;

impl Index {
  pub(crate) fn latest_block(rtx: &Rtx) -> Result<Option<(Height, BlockHash)>> {
    Ok(
      rtx
        .0
        .open_table(HEIGHT_TO_BLOCK_HEADER)?
        .range(0..)?
        .next_back()
        .and_then(|result| result.ok())
        .map(|(height, hash)| {
          (
            Height(height.value()),
            Header::load(*hash.value()).block_hash(),
          )
        }),
    )
  }

  pub(crate) fn get_tx(txid: Txid, rtx: &Rtx, index: &Index) -> Result<Option<Transaction>> {
    if txid == index.genesis_block_coinbase_txid {
      return Ok(Some(index.genesis_block_coinbase_transaction.clone()));
    }

    if index.index_transactions {
      if let Some(transaction) = rtx
        .0
        .open_table(TRANSACTION_ID_TO_TRANSACTION)?
        .get(&txid.store())?
      {
        return Ok(Some(consensus::encode::deserialize(transaction.value())?));
      }
    }

    index.client.get_raw_transaction(&txid, None).into_option()
  }
}
