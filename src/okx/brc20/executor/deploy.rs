use super::*;
use std::u128;

impl BRC20ExecutionMessage {
  pub(super) fn execute_deploy(
    &self,
    context: &mut TableContext,
    height: u32,
    blocktime: u32,
  ) -> Result<BRC20Receipt, ExecutionError> {
    let BRC20Operation::Deploy(deploy) = &self.operation else {
      unreachable!()
    };

    // get the deployer address.
    let deployer = self.receiver.clone().unwrap();
    let ticker = BRC20Ticker::from_str(&deploy.tick).map_err(BRC20Error::TickerParse)?;

    // check if the ticker is not already deployed.
    if context.load_brc20_ticker_info(&ticker)?.is_some() {
      return Err(ExecutionError::ExecutionFailed(
        BRC20Error::DuplicateDeployment(ticker.to_string()),
      ));
    }

    // validate and parse the decimals.
    let decimals = if let Some(dec) = &deploy.decimals {
      let uncheck = FixedPoint::new_from_str(dec, 0).map_err(BRC20Error::NumericError)?;
      let (value, scale) = uncheck.to_u128_and_scale();
      if scale != 0 || value > u128::from(FixedPoint::MAX_SCALE) {
        return Err(ExecutionError::ExecutionFailed(
          BRC20Error::DecimalsExceedLimit(uncheck),
        ));
      } else {
        u8::try_from(value).unwrap()
      }
    } else {
      FixedPoint::MAX_SCALE
    };

    // validate and parse the max supply.
    let mut max =
      FixedPoint::new_from_str(&deploy.max_supply, decimals).map_err(BRC20Error::NumericError)?;
    if max > *MAXIMUM_SUPPLY {
      return Err(ExecutionError::ExecutionFailed(BRC20Error::InvalidSupply(
        max,
      )));
    }

    let self_minted = deploy.self_mint.unwrap_or_default();
    if max.is_zero() {
      if self_minted {
        max = FixedPoint::new_unchecked(
          u128::from(u64::MAX) * 10_u128.pow(decimals as u32),
          decimals,
        );
      } else {
        return Err(ExecutionError::ExecutionFailed(BRC20Error::InvalidSupply(
          max,
        )));
      }
    }

    // limit the mint amount.
    let limit = if let Some(lim) = &deploy.mint_limit {
      let limit = FixedPoint::new_from_str(lim, decimals).map_err(BRC20Error::NumericError)?;
      if limit > *MAXIMUM_SUPPLY {
        return Err(ExecutionError::ExecutionFailed(
          BRC20Error::InvalidMaxMintLimit(limit),
        ));
      }
      if limit.is_zero() {
        if self_minted {
          FixedPoint::new(u128::from(u64::MAX), decimals).unwrap()
        } else {
          return Err(ExecutionError::ExecutionFailed(
            BRC20Error::InvalidMaxMintLimit(limit),
          ));
        }
      } else {
        limit
      }
    } else {
      max
    };

    let total_supply = max.to_u128_and_scale().0;
    let max_mint_limit = limit.to_u128_and_scale().0;

    let ticker_info = BRC20TickerInfo {
      ticker: ticker.clone(),
      sequence_number: self.sequence_number,
      inscription_number: self.inscription_number,
      inscription_id: self.inscription_id,
      total_supply,
      burned: 0,
      minted: 0,
      max_mint_limit,
      decimals,
      deployer: deployer.clone(),
      self_minted,
      deployed_block_height: height,
      deployed_timestamp: blocktime,
      latest_minted_block_height: height,
    };

    // insert the ticker info to the table.
    context.update_brc20_ticker_info(&ticker, ticker_info)?;

    Ok(BRC20Receipt {
      inscription_id: self.inscription_id,
      sequence_number: self.sequence_number,
      inscription_number: self.inscription_number,
      old_satpoint: self.old_satpoint,
      new_satpoint: self.new_satpoint,
      sender: self.sender.clone(),
      receiver: deployer,
      op_type: BRC20OpType::Deploy,
      result: Ok(BRC20Event::Deploy(DeployEvent {
        ticker,
        total_supply,
        decimals,
        self_minted,
        max_mint_limit,
      })),
    })
  }
}
