use crate::index::simulator::processor::StorageProcessor;
use crate::index::updater::inscription_updater::{Flotsam, Origin};
use {
  super::*,
  crate::okx::datastore::ord::operation::{Action, InscriptionOp},
};

pub struct PendingUpdater<'a, 'db, 'tx> {
  pub processor: StorageProcessor<'a, 'db, 'tx>,
  pub(super) operations: &'a mut HashMap<Txid, Vec<InscriptionOp>>,
  pub(super) blessed_inscription_count: u64,
  pub(super) chain: Chain,
  pub(super) cursed_inscription_count: u64,
  pub(super) height: u32,
  pub(super) home_inscription_count: u64,
  pub(super) index_transactions: bool,
  pub(super) next_sequence_number: u32,
  pub(super) lost_sats: u64,
  pub(super) reward: u64,
  pub(super) transaction_buffer: Vec<u8>,
  pub(super) timestamp: u32,
  pub(super) unbound_inscriptions: u64,
  pub(super) tx_out_cache: &'a mut SimpleLru<OutPoint, TxOut>,
  pub(super) new_outpoints: Vec<OutPoint>,
}

impl<'a, 'db, 'tx> PendingUpdater<'a, 'db, 'tx> {
  pub fn new(
    operations: &'a mut HashMap<Txid, Vec<InscriptionOp>>,
    blessed_inscription_count: u64,
    chain: Chain,
    cursed_inscription_count: u64,
    height: u32,
    index_transactions: bool,
    next_sequence_number: u32,
    lost_sats: u64,
    timestamp: u32,
    unbound_inscriptions: u64,
    tx_out_cache: &'a mut SimpleLru<OutPoint, TxOut>,
    processor: StorageProcessor<'a, 'db, 'tx>,
  ) -> Result<Self> {
    let home_inscriptions_len = processor.home_inscriptions_len();
    Ok(Self {
      processor,
      operations,
      blessed_inscription_count,
      chain,
      cursed_inscription_count,
      height,
      home_inscription_count: home_inscriptions_len,
      index_transactions,
      next_sequence_number,
      lost_sats,
      reward: Height(height).subsidy(),
      transaction_buffer: vec![],
      timestamp,
      unbound_inscriptions,
      tx_out_cache,
      new_outpoints: vec![],
    })
  }
  pub fn index_envelopes(
    &mut self,
    tx: &Transaction,
    txid: Txid,
    input_sat_ranges: Option<&VecDeque<(u64, u64)>>,
  ) -> Result {
    let mut floating_inscriptions = Vec::new();
    let mut id_counter = 0;
    let mut inscribed_offsets = BTreeMap::new();
    let jubilant = self.height >= self.chain.jubilee_height();
    let mut total_input_value = 0;
    let total_output_value = tx.output.iter().map(|txout| txout.value).sum::<u64>();

    let envelopes = ParsedEnvelope::from_transaction(tx);
    let inscriptions = !envelopes.is_empty();
    let mut envelopes = envelopes.into_iter().peekable();

    for (input_index, tx_in) in tx.input.iter().enumerate() {
      // skip subsidy since no inscriptions possible
      if tx_in.previous_output.is_null() {
        total_input_value += Height(self.height).subsidy();
        continue;
      }

      // find existing inscriptions on input (transfers of inscriptions)
      for (old_satpoint, inscription_id) in self
        .processor
        .inscriptions_on_output(&tx_in.previous_output)?
      {
        let offset = total_input_value + old_satpoint.offset;
        floating_inscriptions.push(Flotsam {
          txid,
          offset,
          inscription_id,
          old_satpoint,
          origin: Origin::Old,
        });

        inscribed_offsets
          .entry(offset)
          .or_insert((inscription_id, 0))
          .1 += 1;
      }

      let offset = total_input_value;

      // multi-level cache for UTXO set to get to the input amount
      let current_input_value = if let Some(tx_out) = self.tx_out_cache.get(&tx_in.previous_output)
      {
        tx_out.value
      } else {
        panic!("tx_out_cache should have tx_out for input");
      };

      total_input_value += current_input_value;

      // go through all inscriptions in this input
      while let Some(inscription) = envelopes.peek() {
        if inscription.input != u32::try_from(input_index).unwrap() {
          break;
        }

        let inscription_id = InscriptionId {
          txid,
          index: id_counter,
        };

        let curse = if inscription.payload.unrecognized_even_field {
          Some(crate::index::updater::inscription_updater::Curse::UnrecognizedEvenField)
        } else if inscription.payload.duplicate_field {
          Some(crate::index::updater::inscription_updater::Curse::DuplicateField)
        } else if inscription.payload.incomplete_field {
          Some(crate::index::updater::inscription_updater::Curse::IncompleteField)
        } else if inscription.input != 0 {
          Some(crate::index::updater::inscription_updater::Curse::NotInFirstInput)
        } else if inscription.offset != 0 {
          Some(crate::index::updater::inscription_updater::Curse::NotAtOffsetZero)
        } else if inscription.payload.pointer.is_some() {
          Some(crate::index::updater::inscription_updater::Curse::Pointer)
        } else if inscription.pushnum {
          Some(crate::index::updater::inscription_updater::Curse::Pushnum)
        } else if inscription.stutter {
          Some(crate::index::updater::inscription_updater::Curse::Stutter)
        } else if let Some((id, count)) = inscribed_offsets.get(&offset) {
          if *count > 1 {
            Some(crate::index::updater::inscription_updater::Curse::Reinscription)
          } else {
            let initial_inscription_sequence_number = self
              .processor
              .id_to_sequence_number_get(&id.store())?
              .unwrap();

            let entry = InscriptionEntry::load(
              self
                .processor
                .sequence_number_to_entry_get(initial_inscription_sequence_number)?
                .unwrap(),
            );

            let initial_inscription_was_cursed_or_vindicated =
              entry.inscription_number < 0 || Charm::Vindicated.is_set(entry.charms);

            if initial_inscription_was_cursed_or_vindicated {
              None
            } else {
              Some(crate::index::updater::inscription_updater::Curse::Reinscription)
            }
          }
        } else {
          None
        };

        let unbound = current_input_value == 0
          || curse
            == Some(crate::index::updater::inscription_updater::Curse::UnrecognizedEvenField)
          || inscription.payload.unrecognized_even_field;

        let offset = inscription
          .payload
          .pointer()
          .filter(|&pointer| pointer < total_output_value)
          .unwrap_or(offset);

        floating_inscriptions.push(Flotsam {
          txid,
          inscription_id,
          offset,
          old_satpoint: SatPoint {
            outpoint: tx_in.previous_output,
            offset: 0,
          },
          origin: Origin::New {
            cursed: curse.is_some() && !jubilant,
            fee: 0,
            hidden: inscription.payload.hidden(),
            parent: inscription.payload.parent(),
            pointer: inscription.payload.pointer(),
            reinscription: inscribed_offsets.get(&offset).is_some(),
            unbound,
            inscription: inscription.payload.clone(),
            vindicated: curse.is_some() && jubilant,
          },
        });

        inscribed_offsets
          .entry(offset)
          .or_insert((inscription_id, 0))
          .1 += 1;

        envelopes.next();
        id_counter += 1;
      }
    }

    if self.index_transactions && inscriptions {
      tx.consensus_encode(&mut self.transaction_buffer)
        .expect("in-memory writers don't error");
      self
        .processor
        .transaction_id_to_transaction_insert(&txid.store(), self.transaction_buffer.as_slice())?;

      self.transaction_buffer.clear();
    }

    let potential_parents = floating_inscriptions
      .iter()
      .map(|flotsam| flotsam.inscription_id)
      .collect::<HashSet<InscriptionId>>();

    for flotsam in &mut floating_inscriptions {
      if let Flotsam {
        origin: Origin::New { parent, .. },
        ..
      } = flotsam
      {
        if let Some(purported_parent) = parent {
          if !potential_parents.contains(purported_parent) {
            *parent = None;
          }
        }
      }
    }

    // still have to normalize over inscription size
    for flotsam in &mut floating_inscriptions {
      if let Flotsam {
        origin: Origin::New { ref mut fee, .. },
        ..
      } = flotsam
      {
        *fee = (total_input_value - total_output_value) / u64::from(id_counter);
      }
    }

    floating_inscriptions.sort_by_key(|flotsam| flotsam.offset);
    let mut inscriptions = floating_inscriptions.into_iter().peekable();

    let mut range_to_vout = BTreeMap::new();
    let mut new_locations = Vec::new();
    let mut output_value = 0;
    for (vout, tx_out) in tx.output.iter().enumerate() {
      let end = output_value + tx_out.value;

      while let Some(flotsam) = inscriptions.peek() {
        if flotsam.offset >= end {
          break;
        }

        let new_satpoint = SatPoint {
          outpoint: OutPoint {
            txid,
            vout: vout.try_into().unwrap(),
          },
          offset: flotsam.offset - output_value,
        };

        new_locations.push((new_satpoint, inscriptions.next().unwrap()));
      }

      range_to_vout.insert((output_value, end), vout.try_into().unwrap());

      output_value = end;

      self.tx_out_cache.insert(
        OutPoint {
          vout: vout.try_into().unwrap(),
          txid,
        },
        tx_out.clone(),
      );
    }

    for (new_satpoint, mut flotsam) in new_locations.into_iter() {
      let new_satpoint = match flotsam.origin {
        Origin::New {
          pointer: Some(pointer),
          ..
        } if pointer < output_value => {
          match range_to_vout.iter().find_map(|((start, end), vout)| {
            (pointer >= *start && pointer < *end).then(|| (vout, pointer - start))
          }) {
            Some((vout, offset)) => {
              flotsam.offset = pointer;
              SatPoint {
                outpoint: OutPoint { txid, vout: *vout },
                offset,
              }
            }
            _ => new_satpoint,
          }
        }
        _ => new_satpoint,
      };

      self.update_inscription_location(input_sat_ranges, flotsam, new_satpoint)?;
    }

    for flotsam in inscriptions {
      let new_satpoint = SatPoint {
        outpoint: OutPoint::null(),
        offset: u64::MAX,
      };
      self.update_inscription_location(input_sat_ranges, flotsam, new_satpoint)?;
    }
    self.lost_sats += self.reward - output_value;
    Ok(())
  }

  // write tx_out to outpoint_to_entry table
  pub fn flush_cache(self) -> Result {
    let start = Instant::now();
    let persist = self.new_outpoints.len();
    let mut entry = Vec::new();
    for outpoint in self.new_outpoints.into_iter() {
      let tx_out = self.tx_out_cache.get(&outpoint).unwrap();
      tx_out.consensus_encode(&mut entry)?;
      self
        .processor
        .outpoint_to_entry_insert(&outpoint.store(), entry.as_slice())?;
      entry.clear();
    }
    log::info!(
      "flush cache, persist:{}, global:{} cost: {}ms",
      persist,
      self.tx_out_cache.len(),
      start.elapsed().as_millis()
    );
    Ok(())
  }

  fn calculate_sat(
    input_sat_ranges: Option<&VecDeque<(u64, u64)>>,
    input_offset: u64,
  ) -> Option<Sat> {
    let input_sat_ranges = input_sat_ranges?;

    let mut offset = 0;
    for (start, end) in input_sat_ranges {
      let size = end - start;
      if offset + size > input_offset {
        let n = start + input_offset - offset;
        return Some(Sat(n));
      }
      offset += size;
    }

    unreachable!()
  }

  fn update_inscription_location(
    &mut self,
    input_sat_ranges: Option<&VecDeque<(u64, u64)>>,
    flotsam: Flotsam,
    new_satpoint: SatPoint,
  ) -> Result {
    let inscription_id = flotsam.inscription_id;
    let (unbound, sequence_number) = match flotsam.origin {
      Origin::Old => {
        self
          .processor
          .satpoint_to_sequence_number_remove_all(&flotsam.old_satpoint.store())?;

        (
          false,
          self
            .processor
            .id_to_sequence_number_get(&inscription_id.store())?
            .unwrap(),
        )
      }
      Origin::New {
        cursed,
        fee,
        hidden,
        parent,
        pointer: _,
        reinscription,
        unbound,
        inscription: _,
        vindicated,
      } => {
        let inscription_number = if cursed {
          let number: i32 = self.cursed_inscription_count.try_into().unwrap();
          self.cursed_inscription_count += 1;

          // because cursed numbers start at -1
          -(number + 1)
        } else {
          let number: i32 = self.blessed_inscription_count.try_into().unwrap();
          self.blessed_inscription_count += 1;

          number
        };

        let sequence_number = self.next_sequence_number;
        self.next_sequence_number += 1;

        self
          .processor
          .inscription_number_to_sequence_number_insert(inscription_number, sequence_number)?;

        let sat = if unbound {
          None
        } else {
          Self::calculate_sat(input_sat_ranges, flotsam.offset)
        };

        let mut charms = 0;

        if cursed {
          Charm::Cursed.set(&mut charms);
        }

        if reinscription {
          Charm::Reinscription.set(&mut charms);
        }

        if let Some(sat) = sat {
          if sat.nineball() {
            Charm::Nineball.set(&mut charms);
          }

          if sat.coin() {
            Charm::Coin.set(&mut charms);
          }

          match sat.rarity() {
            Rarity::Common | Rarity::Mythic => {}
            Rarity::Uncommon => Charm::Uncommon.set(&mut charms),
            Rarity::Rare => Charm::Rare.set(&mut charms),
            Rarity::Epic => Charm::Epic.set(&mut charms),
            Rarity::Legendary => Charm::Legendary.set(&mut charms),
          }
        }

        if new_satpoint.outpoint == OutPoint::null() {
          Charm::Lost.set(&mut charms);
        }

        if unbound {
          Charm::Unbound.set(&mut charms);
        }

        if vindicated {
          Charm::Vindicated.set(&mut charms);
        }

        if let Some(Sat(n)) = sat {
          self
            .processor
            .sat_to_sequence_number_insert(&n, &sequence_number)?;
        }

        let parent = match parent {
          Some(parent_id) => {
            let parent_sequence_number = self
              .processor
              .id_to_sequence_number_get(&parent_id.store())?
              .unwrap();
            self
              .processor
              .sequence_number_to_children_insert(parent_sequence_number, sequence_number)?;

            Some(parent_sequence_number)
          }
          None => None,
        };

        self.processor.sequence_number_to_entry_insert(
          sequence_number,
          &InscriptionEntry {
            charms,
            fee,
            height: self.height,
            id: inscription_id,
            inscription_number,
            parent,
            sat,
            sequence_number,
            timestamp: self.timestamp,
          }
          .store(),
        )?;

        self
          .processor
          .id_to_sequence_number_insert(&inscription_id.store(), sequence_number)?;

        if !hidden {
          self
            .processor
            .home_inscriptions_insert(&sequence_number, inscription_id.store())?;

          if self.home_inscription_count == 100 {
            self.processor.home_inscriptions_pop_first()?;
          } else {
            self.home_inscription_count += 1;
          }
        }

        (unbound, sequence_number)
      }
    };

    let satpoint = if unbound {
      let new_unbound_satpoint = SatPoint {
        outpoint: unbound_outpoint(),
        offset: self.unbound_inscriptions,
      };
      self.unbound_inscriptions += 1;
      new_unbound_satpoint.store()
    } else {
      new_satpoint.store()
    };
    let mut is_transfer_to_coinbase = false;
    let point = if new_satpoint.outpoint.is_null() && new_satpoint.offset == u64::MAX {
      is_transfer_to_coinbase = true;
      None
    } else {
      Some(new_satpoint)
    };

    self
      .operations
      .entry(flotsam.txid)
      .or_default()
      .push(InscriptionOp {
        txid: flotsam.txid,
        sequence_number,
        inscription_number: self
          .processor
          .sequence_number_to_entry_get(sequence_number)?
          .map(|entry| InscriptionEntry::load(entry).inscription_number),
        inscription_id: flotsam.inscription_id,
        action: match flotsam.origin {
          Origin::Old => Action::Transfer,
          Origin::New {
            cursed,
            fee: _,
            hidden: _,
            parent: _,
            pointer: _,
            reinscription: _,
            unbound,
            inscription,
            vindicated,
          } => Action::New {
            cursed,
            unbound,
            vindicated,
            inscription,
            transfer_to_coin_base: is_transfer_to_coinbase,
          },
        },
        old_satpoint: flotsam.old_satpoint,
        new_satpoint: point,
      });

    self
      .processor
      .satpoint_to_sequence_number_insert(&satpoint, sequence_number)?;
    self
      .processor
      .sequence_number_to_satpoint_insert(sequence_number, &satpoint)?;

    Ok(())
  }
}
