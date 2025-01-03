use super::*;
use crate::index::entry::SatPointValue;
use crate::index::event::Action;
use crate::index::event::OkxInscriptionEvent;
use crate::index::{BlockData, BundleMessage, Curse};
use crate::okx::bitmap::BitmapMessage;
use crate::okx::brc20::entry::BRC20TransferAssetValue;
use crate::okx::brc20::{BRC20ExecutionMessage, BRC20Message};
use crate::okx::context::TableContext;
use crate::okx::entry::{AddressTickerKeyValue, InscriptionReceipt};
use redb::{MultimapTable, Table};
use std::collections::HashMap;

pub(crate) mod bitmap;
pub(crate) mod brc20;
mod composite_key;
pub(crate) mod context;
pub(crate) mod entry;
mod utxo_address;

pub(crate) use self::{
  composite_key::{AddressEndpoint, AddressTickerKey},
  utxo_address::{UtxoAddress, UtxoAddressRef},
};

#[derive(Debug, Clone)]
pub enum SubMessage {
  BRC20(BRC20Message),
  BITMAP(BitmapMessage),
}

pub(crate) struct OkxUpdater {
  pub(crate) height: u32,
  pub(crate) timestamp: u32,
  pub(crate) chain: Chain,
  pub(crate) save_inscription_receipts: bool,
}

impl OkxUpdater {
  pub(crate) fn index_block_bundle_messages(
    &mut self,
    context: &mut TableContext,
    block: &BlockData,
    mut block_bundle_messages: HashMap<Txid, Vec<BundleMessage>>,
  ) -> Result<()> {
    log::info!("Indexing BRC20 block at height {}", self.height);

    for (tx_offset, (tx, txid)) in block
      .txdata
      .iter()
      .enumerate()
      .skip(1)
      .chain(block.txdata.iter().enumerate().take(1))
    {
      let Some(mut tx_bundle_messages) = block_bundle_messages.remove(txid) else {
        continue;
      };

      tx_bundle_messages.sort_by_key(|msg| msg.offset);

      let mut brc20_tx_receipts = Vec::new();

      for bundle_message in tx_bundle_messages.iter() {
        if let Some(brc20_message) = Option::<BRC20ExecutionMessage>::from(bundle_message) {
          if let Ok(receipt) =
            brc20_message.execute(context, self.height, self.chain, self.timestamp)
          {
            brc20_tx_receipts.push(receipt);
          }
        } else if let Some(SubMessage::BITMAP(bitmap_message)) = &bundle_message.sub_message {
          bitmap_message.execute(context, self.height)?;
        }
      }

      // save brc20_tx_receipts
      if !brc20_tx_receipts.is_empty() {
        context.insert_brc20_tx_receipts(txid, brc20_tx_receipts)?;
      }

      if self.save_inscription_receipts {
        let inscription_receipts = tx_bundle_messages
          .into_iter()
          .map(|msg| msg.into())
          .collect();
        context.insert_inscription_tx_receipts(txid, inscription_receipts)?;
      }
    }
    Ok(())
  }
}
