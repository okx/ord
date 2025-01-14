use super::*;

// ord/debug/bitmap/district/:number
pub(crate) async fn ord_debug_bitmap_district(
  Extension(index): Extension<Arc<Index>>,
  Path(number): Path<u32>,
) -> ApiResult<InscriptionId> {
  log::debug!("rpc: get ord_debug_bitmap_district: number:{}", number);

  task::block_in_place(|| {
    let rtx = index.begin_read()?;
    let sequence_number = Index::get_bitmap_by_block_height(number, &rtx)?
      .ok_or_api_not_found(format!("district {number} not found."))?;

    let inscription_entry = Index::inscription_entry_by_sequence_number(sequence_number, &rtx)?
      .ok_or_api_not_found(format!("district {number} not found."))?;

    log::debug!(
      "rpc: get ord_debug_bitmap_district: {:?} {:?}",
      number,
      sequence_number
    );

    Ok(Json(ApiResponse::ok(inscription_entry.id)))
  })
}
