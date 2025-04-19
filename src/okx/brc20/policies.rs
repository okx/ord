use crate::{index::Curse, Chain};
use ordinals::Charm;

pub struct HardForks;

impl HardForks {
  /// Proposed block activation height for issuance and burn enhancements.
  /// Proposal content: https://l1f.discourse.group/t/brc-20-proposal-for-issuance-and-burn-enhancements-brc20-ip-1/621
  pub fn self_issuance_activation_height(chain: &Chain) -> u32 {
    match chain {
      Chain::Mainnet => 837090,  // decided by community
      Chain::Testnet => 2413343, // decided by okx team
      Chain::Regtest => 0,
      Chain::Signet => 0,
      Chain::Testnet4 => 0,
    }
  }

  pub fn self_single_step_transfer_activation_height(chain: &Chain) -> u32 {
    match chain {
      Chain::Mainnet => 895090,  // decided by community
      Chain::Testnet => 2413343, //
      Chain::Regtest => 0,
      Chain::Signet => 0,
      Chain::Testnet4 => 0,
    }
  }

  /// Check if the inscription preconditions are met for the given curse, charms, height, and chain.
  pub fn check_inscription_preconditions(
    height: u32,
    chain: &Chain,
    charms: u16,
  ) -> bool {
    // can not be unbound or cursed
    if Charm::Unbound.is_set(charms) || Charm::Cursed.is_set(charms) {
      return false;
    }

    let below_activation_height = height < Self::self_single_step_transfer_activation_height(chain);
    if below_activation_height {
      !Charm::Vindicated.is_set(charms)
    } else {
      true
    }
  }
}
