use super::{rtx::Rtx, *};

impl Index {
  pub(crate) fn get_inscription_satpoint_by_id_with_rtx(
    inscription_id: InscriptionId,
    rtx: &Rtx,
  ) -> Result<Option<SatPoint>> {
    let Some(sequence_number) = rtx.inscription_id_to_sequence_number(inscription_id)? else {
      return Ok(None);
    };

    rtx.sequence_number_to_satpoint(sequence_number)
  }

  pub(crate) fn get_inscription_entry_with_rtx(
    inscription_id: InscriptionId,
    rtx: &Rtx,
  ) -> Result<Option<InscriptionEntry>> {
    let Some(sequence_number) = rtx.inscription_id_to_sequence_number(inscription_id)? else {
      return Ok(None);
    };

    rtx.sequence_number_to_inscription_entry(sequence_number)
  }

  pub(crate) fn get_inscription_id_by_inscription_number_with_rtx(
    inscription_number: i32,
    rtx: &Rtx,
  ) -> Result<Option<InscriptionId>> {
    let Some(sequence_number) = rtx.inscription_number_to_sequence_number(inscription_number)?
    else {
      return Ok(None);
    };

    Ok(
      rtx
        .sequence_number_to_inscription_entry(sequence_number)?
        .map(|entry| entry.id),
    )
  }

  pub(crate) fn get_transaction_with_rtx(
    txid: Txid,
    rtx: &Rtx,
    client: &Client,
    network: Network,
    index_transactions: bool,
  ) -> Result<Option<Transaction>> {
    let genesis_block = bitcoin::blockdata::constants::genesis_block(network);
    let genesis_block_coinbase_transaction = genesis_block.coinbase().unwrap();

    if txid == genesis_block_coinbase_transaction.txid() {
      return Ok(Some(genesis_block_coinbase_transaction.clone()));
    }

    if index_transactions {
      if let Some(transaction) = rtx.transaction_id_to_transaction(txid)? {
        return Ok(Some(transaction));
      }
    }

    client.get_raw_transaction(&txid, None).into_option()
  }

  pub(crate) fn get_ord_inscription_operations(
    txid: Txid,
    rtx: &Rtx,
    client: &Client,
  ) -> Result<Option<Vec<ord::InscriptionOp>>> {
    let Some(operations) = rtx.ord_transaction_id_to_inscription_operations(txid)? else {
      let raw_tx = client.get_raw_transaction_info(&txid, None)?;

      match raw_tx.blockhash {
        Some(tx_blockhash) => {
          // Get the block header of the transaction. We should check if the block has been parsed by the indexer.
          let tx_bh = client.get_block_header_info(&tx_blockhash)?;

          // Check if the block hash has been parsed by the indexer.
          // If it has been parsed, proceed to the next step.
          let Some(parsed_hash) = rtx.block_hash(Some(u32::try_from(tx_bh.height).unwrap()))?
          else {
            // If it has not been parsed, return None.
            return Ok(None);
          };

          // Check if the block hash of the parsed transaction is the same as the indexed parsed blocks.
          if parsed_hash != tx_blockhash {
            // In the different conflicting block.
            return Ok(None);
          }
          // Empty inscription operations in the transaction.
          return Ok(Some(Vec::new()));
        }
        None => {
          return Err(anyhow!(
            "Can't retrieve pending transaction operations. {txid}"
          ))
        }
      }
    };
    Ok(Some(operations))
  }

  pub(crate) fn get_ord_block_inscription_operations(
    block_hash: BlockHash,
    rtx: &Rtx,
    client: &Client,
  ) -> Result<Vec<(bitcoin::Txid, Vec<ord::InscriptionOp>)>> {
    // get block from btc client.
    let blockinfo = client.get_block_info(&block_hash)?;

    // get blockhash from redb.
    let Some(block_hash) = rtx.block_hash(Some(u32::try_from(blockinfo.height).unwrap()))? else {
      return Err(anyhow!(
        "Can't retrieve block: {} from the database.",
        blockinfo.height
      ));
    };

    // check of conflicting block.
    if blockinfo.hash != block_hash {
      return Err(anyhow!(
        "Conflict with block hash in the database. {} != {}",
        block_hash,
        blockinfo.hash
      ));
    }

    let mut result = Vec::new();
    for txid in blockinfo.tx {
      let Some(inscriptions) = rtx.ord_transaction_id_to_inscription_operations(txid)? else {
        continue;
      };
      result.push((txid, inscriptions));
    }
    Ok(result)
  }

  pub(crate) fn get_brc20_balance_by_tick_and_address(
    tick: brc20::Tick,
    script_key: ScriptKey,
    rtx: &Rtx,
  ) -> Result<Option<brc20::Balance>> {
    Ok(match rtx.brc20_get_balance_by_address(&tick, script_key)? {
      Some(balance) => Some(balance),
      None if rtx.brc20_get_tick_info(&tick)?.is_some() => Some(brc20::Balance {
        tick: tick.clone(),
        overall_balance: 0,
        transferable_balance: 0,
      }),
      _ => None,
    })
  }

  pub(crate) fn get_brc20_transferable_utxo_by_tick_and_address(
    tick: brc20::Tick,
    script_key: ScriptKey,
    rtx: &Rtx,
  ) -> Result<Option<Vec<(SatPoint, brc20::TransferableLog)>>> {
    let transferable_utxo_assets = rtx.brc20_get_tick_transferable_by_address(&tick, script_key)?;

    if transferable_utxo_assets.is_empty() {
      if rtx.brc20_get_tick_info(&tick)?.is_some() {
        return Ok(Some(Vec::new()));
      } else {
        return Ok(None);
      }
    }
    Ok(Some(transferable_utxo_assets))
  }

  pub(crate) fn get_brc20_transaction_receipts(
    txid: Txid,
    rtx: &Rtx,
    client: &Client,
  ) -> Result<Option<Vec<brc20::Receipt>>> {
    let Some(receipts) = rtx.brc20_transaction_id_to_transaction_receipt(txid)? else {
      let raw_tx = client.get_raw_transaction_info(&txid, None)?;

      match raw_tx.blockhash {
        Some(tx_blockhash) => {
          // Get the block header of the transaction. We should check if the block has been parsed by the indexer.
          let tx_bh = client.get_block_header_info(&tx_blockhash)?;

          // Check if the block hash has been parsed by the indexer.
          // If it has been parsed, proceed to the next step.
          let Some(parsed_hash) = rtx.block_hash(Some(u32::try_from(tx_bh.height).unwrap()))?
          else {
            // If it has not been parsed, return None.
            return Ok(None);
          };

          // Check if the block hash of the parsed transaction is the same as the indexed parsed blocks.
          if parsed_hash != tx_blockhash {
            // In the different conflicting block.
            return Ok(None);
          }
          // Empty inscription operations in the transaction.
          return Ok(Some(Vec::new()));
        }
        None => {
          return Err(anyhow!(
            "Can't retrieve pending BRC20 transaction receipts. {txid}"
          ))
        }
      }
    };
    Ok(Some(receipts))
  }

  pub(crate) fn get_brc20_block_receipts(
    block_hash: BlockHash,
    rtx: &Rtx,
    client: &Client,
  ) -> Result<Vec<(bitcoin::Txid, Vec<brc20::Receipt>)>> {
    // get block from btc client.
    let blockinfo = client.get_block_info(&block_hash)?;

    // get blockhash from redb.
    let Some(block_hash) = rtx.block_hash(Some(u32::try_from(blockinfo.height).unwrap()))? else {
      return Err(anyhow!(
        "Can't retrieve block: {} from the database.",
        blockinfo.height
      ));
    };

    // check of conflicting block.
    if blockinfo.hash != block_hash {
      return Err(anyhow!(
        "Conflict with block hash in the database. {} != {}",
        block_hash,
        blockinfo.hash
      ));
    }

    let mut result = Vec::new();
    for txid in blockinfo.tx {
      let Some(inscriptions) = rtx.brc20_transaction_id_to_transaction_receipt(txid)? else {
        continue;
      };
      result.push((txid, inscriptions));
    }
    Ok(result)
  }

  // Assume these are helper functions defined elsewhere in the module.
  pub(crate) fn fetch_vout(
    rtx: &Rtx,
    client: &Client,
    outpoint: OutPoint,
    network: Network,
    index_transactions: bool,
  ) -> Result<Option<TxOut>> {
    // Try to get the txout from the database store at first.
    if let Some(txout) = rtx.outpoint_to_entry(outpoint)? {
      Ok(Some(txout))
    } else {
      // Try to get the txout from the transaction table or the RPC request.
      Ok(
        Self::get_transaction_with_rtx(outpoint.txid, rtx, client, network, index_transactions)?
          .map(|tx| {
            tx.output
              .get(usize::try_from(outpoint.vout).unwrap())
              .unwrap()
              .to_owned()
          }),
      )
    }
  }
}
