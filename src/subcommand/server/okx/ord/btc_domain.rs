use super::*;

// ord/debug/btc_nam/:btc_name
pub(crate) async fn ord_debug_btc_domain(
  Extension(index): Extension<Arc<Index>>,
  Path(btc_domain): Path<String>,
) -> ApiResult<InscriptionId> {
  log::info!("rpc: get ord_debug_btc_domain:{btc_domain}");

  task::block_in_place(|| {
    let rtx = index.begin_read()?;

    let sequence_number = Index::get_btc_domain(&btc_domain, &rtx)?
      .ok_or_api_not_found(format!("btc domain {btc_domain} not found."))?;
    let inscription_entry = Index::inscription_entry_by_sequence_number(sequence_number, &rtx)?
      .ok_or_api_not_found(format!("btc domain {btc_domain} not found."))?;

    log::info!(
      "rpc: get ord_debug_btc_domain: {:?} {:?}",
      btc_domain,
      inscription_entry.id
    );

    Ok(Json(ApiResponse::ok(inscription_entry.id)))
  })
}
