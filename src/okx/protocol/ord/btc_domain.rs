use crate::okx::datastore::ord::{OrdReader, OrdReaderWriter};
use crate::okx::protocol::context::Context;
use {
  crate::{
    okx::datastore::ord::{
      btc_domain::BtcDomain,
      collections::CollectionKind,
      operation::{Action, InscriptionOp},
    },
    Inscription, InscriptionId, Result,
  },
  anyhow::anyhow,
  bitcoin::Txid,
  std::collections::HashMap,
};

pub fn index_btc_domain(
  context: &mut Context,
  operations: &HashMap<Txid, Vec<InscriptionOp>>,
  domain_list: &[String],
) -> Result<u64> {
  let mut count = 0;

  // ignore transferred or cursed inscriptions.
  let mut positive_inscriptions = operations
    .values()
    .flatten()
    .filter(|op| {
      !op.inscription_number.unwrap().is_negative() && matches!(op.action, Action::New { .. })
    })
    .cloned()
    .collect::<Vec<_>>();

  // sort by inscription number.
  positive_inscriptions.sort_by_key(|op| op.inscription_number.unwrap());

  for op in positive_inscriptions.into_iter() {
    match op.action {
      Action::New { inscription, .. } => {
        if let Some((inscription_id, district)) =
          index_district(context, inscription, op.inscription_id, domain_list)?
        {
          let key = district.to_collection_key();
          context.set_inscription_by_collection_key(&key, &inscription_id)?;
          context.add_inscription_attributes(&inscription_id, CollectionKind::Domain)?;
          count += 1;
        }
      }
      _ => unreachable!(),
    }
  }
  Ok(count)
}

fn index_district(
  context: &mut Context,
  inscription: Inscription,
  inscription_id: InscriptionId,
  domain_list: &[String],
) -> Result<Option<(InscriptionId, BtcDomain)>> {
  if let Some(content) = inscription.body() {
    if let Ok(district) = BtcDomain::parse(content, domain_list) {
      if let Some(h) = district.btc_block_height() {
        if h > context.chain_conf.blockheight {
          return Ok(None);
        }
      }
      let collection_key = district.to_collection_key();

      if context
        .get_collection_inscription_id(&collection_key)
        .map_err(|e| {
          anyhow!("failed to get collection inscription! key: {collection_key} error: {e}")
        })?
        .is_none()
      {
        log::info!(
          "found valid btc domain district! {}.{} inscription_id {}",
          district.name,
          district.domain,
          inscription_id,
        );
        return Ok(Some((inscription_id, district)));
      }
    }
  }
  Ok(None)
}
