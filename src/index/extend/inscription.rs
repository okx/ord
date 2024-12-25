use super::*;

impl Index {
  pub fn inscription_info_by_id(
    inscription_id: InscriptionId,
    rtx: &Rtx,
    index: &Index,
  ) -> Result<Option<(Inscription, InscriptionEntry, SatPoint, Transaction)>> {
    let Some(sequence_number) = rtx
      .0
      .open_table(INSCRIPTION_ID_TO_SEQUENCE_NUMBER)?
      .get(&inscription_id.store())?
      .map(|guard| guard.value())
    else {
      return Ok(None);
    };

    let Some(transaction) = Index::get_tx(inscription_id.txid, rtx, index)? else {
      return Ok(None);
    };

    let Some(inscription) = ParsedEnvelope::from_transaction(&transaction)
      .into_iter()
      .nth(inscription_id.index as usize)
      .map(|envelope| envelope.payload)
    else {
      return Ok(None);
    };

    let inscription_entry = rtx
      .0
      .open_table(SEQUENCE_NUMBER_TO_INSCRIPTION_ENTRY)?
      .get(&sequence_number)?
      .map(|entry| InscriptionEntry::load(entry.value()))
      .unwrap();

    let location = rtx
      .0
      .open_table(SEQUENCE_NUMBER_TO_SATPOINT)?
      .get(&sequence_number)?
      .map(|guard| SatPoint::load(*guard.value()))
      .unwrap();
    Ok(Some((
      inscription,
      inscription_entry,
      location,
      transaction,
    )))
  }

  pub fn inscription_info_by_number(
    inscription_number: i32,
    rtx: &Rtx,
    index: &Index,
  ) -> Result<Option<(Inscription, InscriptionEntry, SatPoint, Transaction)>> {
    let Some(sequence_number) = rtx
      .0
      .open_table(INSCRIPTION_NUMBER_TO_SEQUENCE_NUMBER)?
      .get(inscription_number)?
      .map(|guard| guard.value())
    else {
      return Ok(None);
    };

    let inscription_entry = rtx
      .0
      .open_table(SEQUENCE_NUMBER_TO_INSCRIPTION_ENTRY)?
      .get(&sequence_number)?
      .map(|entry| InscriptionEntry::load(entry.value()))
      .unwrap();

    let Some(transaction) = Index::get_tx(inscription_entry.id.txid, rtx, index)? else {
      return Ok(None);
    };

    let Some(inscription) = ParsedEnvelope::from_transaction(&transaction)
      .into_iter()
      .nth(inscription_entry.id.index as usize)
      .map(|envelope| envelope.payload)
    else {
      return Ok(None);
    };

    let location = rtx
      .0
      .open_table(SEQUENCE_NUMBER_TO_SATPOINT)?
      .get(&sequence_number)?
      .map(|guard| SatPoint::load(*guard.value()))
      .unwrap();
    Ok(Some((
      inscription,
      inscription_entry,
      location,
      transaction,
    )))
  }

  pub fn get_inscriptions_on_output_with_satpoints_and_script_pubkey(
    outpoint: OutPoint,
    rtx: &Rtx,
    index: &Index,
  ) -> Result<(Vec<(SatPoint, InscriptionId, i32)>, u64, Option<ScriptBuf>)> {
    let outpoint_to_utxo_entry = rtx.0.open_table(OUTPOINT_TO_UTXO_ENTRY)?;
    let sequence_number_to_inscription_entry =
      rtx.0.open_table(SEQUENCE_NUMBER_TO_INSCRIPTION_ENTRY)?;

    if !index.index_inscriptions {
      return Ok((Vec::new(), 0, None));
    }

    let Some(utxo_entry) = outpoint_to_utxo_entry.get(&outpoint.store())? else {
      return Ok((Vec::new(), 0, None));
    };
    let parsed_utxo_entry = utxo_entry.value().parse(index);
    let mut inscriptions = parsed_utxo_entry.parse_inscriptions();

    inscriptions.sort_by_key(|(sequence_number, _)| *sequence_number);
    inscriptions
      .into_iter()
      .map(|(sequence_number, offset)| {
        let entry = sequence_number_to_inscription_entry
          .get(sequence_number)?
          .map(|entry| InscriptionEntry::load(entry.value()))
          .unwrap();
        let satpoint = SatPoint { outpoint, offset };
        Ok((satpoint, entry.id, entry.inscription_number))
      })
      .collect::<Result<_>>()
      .map(|vec| {
        (
          vec,
          parsed_utxo_entry.total_value(),
          index.index_addresses.then_some(ScriptBuf::from_bytes(
            parsed_utxo_entry.script_pubkey().to_vec(),
          )),
        )
      })
  }

  pub fn ord_get_raw_receipts(txid: &Txid, rtx: &Rtx) -> Result<Option<Vec<InscriptionReceipt>>> {
    let table = rtx.0.open_table(TRANSACTION_ID_TO_INSCRIPTION_RECEIPTS)?;
    Ok(
      table
        .get(&txid.store())?
        .map(|x| DynamicEntry::load(x.value())),
    )
  }

  pub fn ord_get_transaction_receipts(
    txid: Txid,
    rtx: &Rtx,
    index: &Index,
  ) -> Result<Option<Vec<InscriptionReceipt>>> {
    let Some(receipts) = Index::ord_get_raw_receipts(&txid, rtx)? else {
      let raw_tx = index.client.get_raw_transaction_info(&txid, None)?;

      match raw_tx.blockhash {
        Some(tx_blockhash) => {
          // Get the block header of the transaction. We should check if the block has been parsed by the indexer.
          let tx_bh = index.client.get_block_header_info(&tx_blockhash)?;

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
    Ok(Some(receipts))
  }

  pub(crate) fn ord_get_block_receipts(
    block_hash: BlockHash,
    rtx: &Rtx,
    index: &Index,
  ) -> Result<Vec<(Txid, Vec<InscriptionReceipt>)>> {
    // get block from btc client.
    let blockinfo = index.client.get_block_info(&block_hash)?;

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
      let Some(inscriptions) = Index::ord_get_raw_receipts(&txid, rtx)? else {
        continue;
      };
      result.push((txid, inscriptions));
    }
    Ok(result)
  }
}
