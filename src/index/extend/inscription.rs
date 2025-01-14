use super::*;

impl Index {
  pub(crate) fn sequence_number_by_inscription_id(
    inscription_id: InscriptionId,
    rtx: &Rtx,
  ) -> Result<Option<u32>> {
    Ok(
      rtx
        .0
        .open_table(INSCRIPTION_ID_TO_SEQUENCE_NUMBER)?
        .get(&inscription_id.store())?
        .map(|guard| guard.value()),
    )
  }

  pub(crate) fn sequence_number_by_inscription_number(
    inscription_number: i32,
    rtx: &Rtx,
  ) -> Result<Option<u32>> {
    Ok(
      rtx
        .0
        .open_table(INSCRIPTION_NUMBER_TO_SEQUENCE_NUMBER)?
        .get(&inscription_number)?
        .map(|guard| guard.value()),
    )
  }

  pub(crate) fn inscription_entry_by_sequence_number(
    sequence_number: u32,
    rtx: &Rtx,
  ) -> Result<Option<InscriptionEntry>> {
    Ok(
      rtx
        .0
        .open_table(SEQUENCE_NUMBER_TO_INSCRIPTION_ENTRY)?
        .get(&sequence_number)?
        .map(|entry| InscriptionEntry::load(entry.value())),
    )
  }

  pub(crate) fn location_by_sequence_number(
    sequence_number: u32,
    rtx: &Rtx,
  ) -> Result<Option<SatPoint>> {
    Ok(
      rtx
        .0
        .open_table(SEQUENCE_NUMBER_TO_SATPOINT)?
        .get(&sequence_number)?
        .map(|guard| SatPoint::load(*guard.value())),
    )
  }

  pub(crate) fn get_inscriptions_on_output_with_satpoints_and_script_pubkey(
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

  pub(crate) fn ord_get_raw_receipts(
    txid: &Txid,
    rtx: &Rtx,
  ) -> Result<Option<Vec<InscriptionReceipt>>> {
    let table = rtx.0.open_table(TRANSACTION_ID_TO_INSCRIPTION_RECEIPTS)?;
    Ok(
      table
        .get(&txid.store())?
        .map(|x| DynamicEntry::load(x.value())),
    )
  }

  pub(crate) fn get_inscription_collection_by_sequence_number(
    sequence_number: u32,
    rtx: &Rtx,
  ) -> Result<Option<CollectionType>> {
    Ok(
      rtx
        .0
        .open_table(SEQUENCE_NUMBER_TO_COLLECTION_TYPE)?
        .get(sequence_number)?
        .and_then(|v| CollectionType::try_from(v.value()).ok()),
    )
  }

  pub(crate) fn get_bitmap_by_block_height(height: u32, rtx: &Rtx) -> Result<Option<u32>> {
    Ok(
      rtx
        .0
        .open_table(BITMAP_BLOCK_HEIGHT_TO_SEQUENCE_NUMBER)?
        .get(&height)?
        .map(|v| v.value()),
    )
  }

  pub(crate) fn get_btc_domain(domain: &str, rtx: &Rtx) -> Result<Option<u32>> {
    Ok(
      rtx
        .0
        .open_table(BTC_DOMAIN_TO_SEQUENCE_NUMBER)?
        .get(domain)?
        .map(|v| v.value()),
    )
  }
}
