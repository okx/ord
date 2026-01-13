use {super::*, updater::BlockData};

#[derive(Debug, PartialEq)]
pub(crate) enum Error {
  Recoverable { height: u32, depth: u32 },
  Unrecoverable,
}

impl Display for Error {
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    match self {
      Self::Recoverable { height, depth } => {
        write!(f, "{depth} block deep reorg detected at height {height}")
      }
      Self::Unrecoverable => write!(f, "unrecoverable reorg detected"),
    }
  }
}

impl std::error::Error for Error {}

const MAX_SAVEPOINTS: u32 = 2;
const SAVEPOINT_INTERVAL: u32 = 10;
const CHAIN_TIP_DISTANCE: u32 = 21;

pub(crate) struct Reorg {}

impl Reorg {
  pub(crate) fn detect_reorg_with_brc20(
    ord_processing_height: u32,
    brc20_prog_client: &Brc20ProgClient,
  ) -> Result {
    let ord_committed_height = ord_processing_height.saturating_sub(1);
    let brc20_prog_height = brc20_prog_client.eth_block_number()? as u32;

    if ord_committed_height == brc20_prog_height {
      return Ok(());
    }

    let height_diff = ord_committed_height.abs_diff(brc20_prog_height);

    if brc20_prog_height > ord_committed_height {
      // Abnormal case: prog is ahead of ord
      // This should not happen in normal operation, but if it does,
      // we can simply roll back the prog database without triggering a full reorg
      log::info!(
        "BRC20 prog height {} is ahead of ord committed height {} (difference: {}). Rolling back prog only.",
        brc20_prog_height,
        ord_committed_height,
        height_diff
      );
      brc20_prog_client.brc20_reorg(ord_committed_height as u64)?;
      log::info!(
        "Successfully rolled back BRC20 prog to height {}",
        ord_committed_height
      );
      return Ok(());
    }

    // Normal case: ord is ahead of prog (likely due to crash during commit)
    // Check if ord can be rolled back via savepoints
    let max_recoverable_depth =
      (MAX_SAVEPOINTS - 1) * SAVEPOINT_INTERVAL + (ord_processing_height) % SAVEPOINT_INTERVAL;

    if height_diff > max_recoverable_depth {
      log::error!(
        "Height difference ({}) between ord committed height ({}) and brc20 prog height ({}) exceeds max recoverable depth ({})",
        height_diff,
        ord_committed_height,
        brc20_prog_height,
        max_recoverable_depth
      );
      return Err(anyhow!(Error::Unrecoverable));
    }

    log::info!(
      "ord committed height {} is ahead of brc20 prog height {} (difference: {}), triggering recovery",
      ord_committed_height,
      brc20_prog_height,
      height_diff
    );
    return Err(anyhow!(Error::Recoverable {
      height: ord_processing_height,
      depth: 0,
    }));
  }

  pub(crate) fn detect_reorg(block: &BlockData, height: u32, index: &Index) -> Result {
    let bitcoind_prev_blockhash = block.header.prev_blockhash;

    match index.block_hash(height.checked_sub(1))? {
      Some(index_prev_blockhash) if index_prev_blockhash == bitcoind_prev_blockhash => Ok(()),
      Some(index_prev_blockhash) if index_prev_blockhash != bitcoind_prev_blockhash => {
        let max_recoverable_reorg_depth =
          (MAX_SAVEPOINTS - 1) * SAVEPOINT_INTERVAL + height % SAVEPOINT_INTERVAL;

        for depth in 1..max_recoverable_reorg_depth {
          let index_block_hash = index.block_hash(height.checked_sub(depth))?;
          let bitcoind_block_hash = index
            .client
            .get_block_hash(u64::from(height.saturating_sub(depth)))
            .into_option()?;

          if index_block_hash == bitcoind_block_hash {
            return Err(anyhow!(reorg::Error::Recoverable { height, depth }));
          }
        }

        Err(anyhow!(reorg::Error::Unrecoverable))
      }
      _ => Ok(()),
    }
  }

  pub(crate) fn handle_reorg(index: &Index, height: u32, depth: u32) -> Result {
    log::info!("rolling back database after reorg of depth {depth} at height {height}");

    if let redb::Durability::None = index.durability {
      panic!("set index durability to `Durability::Immediate` to test reorg handling");
    }

    let mut wtx = index.begin_write()?;

    let oldest_savepoint =
      wtx.get_persistent_savepoint(wtx.list_persistent_savepoints()?.min().unwrap())?;

    wtx.restore_savepoint(&oldest_savepoint)?;

    Index::increment_statistic(&wtx, Statistic::Commits, 1)?;
    wtx.commit()?;

    let rolled_back_height = index.begin_read()?.block_count()?;

    log::info!(
      "successfully rolled back database to height {}",
      rolled_back_height
    );

    if index.has_brc20_index() {
      if let Some(brc20_prog_client) = &index.brc20_prog_client {
        let brc20_prog_height = brc20_prog_client.eth_block_number()?;
        log::info!("handling BRC20 prog reorg for height {}", brc20_prog_height);
        if brc20_prog_height > rolled_back_height as u64 {
          brc20_prog_client.brc20_reorg(rolled_back_height as u64)?;
          log::info!(
            "successfully rolled back BRC20 prog height to height {}",
            rolled_back_height
          );
        }
      }
    }

    Ok(())
  }

  pub(crate) fn update_savepoints(index: &Index, height: u32) -> Result {
    if let redb::Durability::None = index.durability {
      return Ok(());
    }

    let height = u64::from(height);

    let last_savepoint_height = index
      .begin_read()?
      .0
      .open_table(STATISTIC_TO_COUNT)?
      .get(&Statistic::LastSavepointHeight.key())?
      .map(|last_savepoint_height| last_savepoint_height.value())
      .unwrap_or(0);

    let blocks = index.client.get_blockchain_info()?.headers;

    if (height < SAVEPOINT_INTERVAL.into()
      || height.saturating_sub(last_savepoint_height) >= SAVEPOINT_INTERVAL.into())
      && blocks.saturating_sub(height) <= CHAIN_TIP_DISTANCE.into()
    {
      let wtx = index.begin_write()?;

      let savepoints = wtx.list_persistent_savepoints()?.collect::<Vec<u64>>();

      if savepoints.len() >= usize::try_from(MAX_SAVEPOINTS).unwrap() {
        wtx.delete_persistent_savepoint(savepoints.into_iter().min().unwrap())?;
      }

      Index::increment_statistic(&wtx, Statistic::Commits, 1)?;
      wtx.commit()?;

      let wtx = index.begin_write()?;

      log::debug!("creating savepoint at height {}", height);
      wtx.persistent_savepoint()?;

      wtx
        .open_table(STATISTIC_TO_COUNT)?
        .insert(&Statistic::LastSavepointHeight.key(), &height)?;

      Index::increment_statistic(&wtx, Statistic::Commits, 1)?;
      wtx.commit()?;
    }

    Ok(())
  }
}
