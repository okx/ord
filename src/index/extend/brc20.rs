use super::*;

impl Index {
  pub(crate) fn brc20_get_ticker_info(
    ticker: &BRC20Ticker,
    rtx: &Rtx,
  ) -> Result<Option<BRC20TickerInfo>> {
    let ticker_info_table = rtx.0.open_table(BRC20_TICKER_ENTRY)?;
    Ok(
      ticker_info_table
        .get(ticker.to_lowercase().store().as_ref())?
        .map(|v| BRC20TickerInfo::load(v.value())),
    )
  }

  pub(crate) fn brc20_get_all_ticker_info(rtx: &Rtx) -> Result<Vec<BRC20TickerInfo>> {
    let ticker_info_table = rtx.0.open_table(BRC20_TICKER_ENTRY)?;
    Ok(
      ticker_info_table
        .range::<&BRC20LowerCaseTickerValue>(..)?
        .flat_map(|result| result.map(|(_, v)| BRC20TickerInfo::load(v.value())))
        .collect(),
    )
  }

  pub(crate) fn brc20_get_balance_by_address_ticker(
    utxo_address: &UtxoAddress,
    ticker: &BRC20Ticker,
    rtx: &Rtx,
  ) -> Result<Option<BRC20Balance>> {
    let balances_table = rtx.0.open_table(BRC20_BALANCES)?;

    Ok(
      balances_table
        .get(
          AddressTickerKey {
            primary: utxo_address.clone(),
            secondary: ticker.to_lowercase().clone(),
          }
          .store()
          .as_ref(),
        )?
        .map(|v| BRC20Balance::load(v.value())),
    )
  }

  pub(crate) fn brc20_get_balances_by_address(
    utxo_address: &UtxoAddress,
    rtx: &Rtx,
  ) -> Result<Vec<BRC20Balance>> {
    let balances_table = rtx.0.open_table(BRC20_BALANCES)?;
    Ok(
      balances_table
        .range(
          AddressEndpoint::Left(utxo_address.clone()).store().as_ref()
            ..=AddressEndpoint::Right(utxo_address.clone())
              .store()
              .as_ref(),
        )?
        .flat_map(|result| result.map(|(_, v)| BRC20Balance::load(v.value())))
        .collect(),
    )
  }

  pub(crate) fn brc20_get_transferring_assets_with_location_by_address_ticker(
    utxo_address: &UtxoAddress,
    ticker: &BRC20Ticker,
    rtx: &Rtx,
  ) -> Result<Vec<(SatPoint, BRC20TransferAsset)>> {
    let transferring_satpoint_table = rtx
      .0
      .open_multimap_table(BRC20_ADDRESS_TICKER_TO_TRANSFER_ASSETS)?;

    let satpoint_assets_table = rtx.0.open_table(BRC20_SATPOINT_TO_TRANSFER_ASSETS)?;

    let mut assets = Vec::new();
    for satpoint in transferring_satpoint_table.get(
      AddressTickerKey {
        primary: utxo_address.clone(),
        secondary: ticker.to_lowercase().clone(),
      }
      .store()
      .as_ref(),
    )? {
      let satpoint = SatPoint::load(*satpoint?.value());
      let asset = satpoint_assets_table
        .get(&satpoint.store())?
        .map(|v| BRC20TransferAsset::load(v.value()))
        .unwrap();
      assets.push((satpoint, asset));
    }
    Ok(assets)
  }

  pub(crate) fn get_brc20_transferring_assets_location_by_address(
    utxo_address: &UtxoAddress,
    rtx: &Rtx,
  ) -> Result<Vec<(SatPoint, BRC20TransferAsset)>> {
    let transferring_satpoint_table = rtx
      .0
      .open_multimap_table(BRC20_ADDRESS_TICKER_TO_TRANSFER_ASSETS)?;

    let satpoint_assets_table = rtx.0.open_table(BRC20_SATPOINT_TO_TRANSFER_ASSETS)?;

    let mut assets = Vec::new();
    for range in transferring_satpoint_table.range(
      AddressEndpoint::Left(utxo_address.clone()).store().as_ref()
        ..=AddressEndpoint::Right(utxo_address.clone())
          .store()
          .as_ref(),
    )? {
      let (_, satpoints) = range?;
      for satpoint in satpoints {
        let satpoint = SatPoint::load(*satpoint?.value());
        let asset = satpoint_assets_table
          .get(&satpoint.store())?
          .map(|v| BRC20TransferAsset::load(v.value()))
          .unwrap();
        assets.push((satpoint, asset));
      }
    }
    Ok(assets)
  }

  pub(crate) fn brc20_get_transferring_assets_with_location_by_outpoint(
    outpoint: OutPoint,
    rtx: &Rtx,
  ) -> Result<Vec<(SatPoint, BRC20TransferAsset)>> {
    let satpoint_assets_table = rtx.0.open_table(BRC20_SATPOINT_TO_TRANSFER_ASSETS)?;
    let mut transferable_assets = Vec::new();
    for range in satpoint_assets_table.range::<&[u8; 44]>(
      &SatPoint {
        outpoint,
        offset: 0,
      }
      .store()..&SatPoint {
        outpoint,
        offset: u64::MAX,
      }
      .store(),
    )? {
      let (satpoint, asset) = range?;
      let satpoint = SatPoint::load(*satpoint.value());
      transferable_assets.push((satpoint, DynamicEntry::load(asset.value())));
    }
    Ok(transferable_assets)
  }

  pub(crate) fn brc20_get_raw_receipts(txid: &Txid, rtx: &Rtx) -> Result<Option<Vec<BRC20Receipt>>> {
    let table = rtx.0.open_table(BRC20_TRANSACTION_ID_TO_RECEIPTS)?;
    Ok(
      table
        .get(&txid.store())?
        .map(|x| DynamicEntry::load(x.value())),
    )
  }
}
