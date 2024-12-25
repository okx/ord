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
    }
  }

  pub fn draft_reinscription_activation_height(chain: &Chain) -> u32 {
    match chain {
      Chain::Mainnet => u32::MAX, // todo: not set yet
      Chain::Testnet => u32::MAX,
      Chain::Regtest => u32::MAX,
      Chain::Signet => u32::MAX,
    }
  }

  /// Check if the inscription preconditions are met for the given curse, charms, height, and chain.
  pub fn check_inscription_preconditions(
    height: u32,
    chain: &Chain,
    charms: u16,
    pre_jubilant_curse_reason: Option<&Curse>,
  ) -> bool {
    // can not be unbound or cursed
    if Charm::Unbound.is_set(charms) || Charm::Cursed.is_set(charms) {
      return false;
    }

    let vindicated_set = Charm::Vindicated.is_set(charms);
    let below_activation_height = height < Self::draft_reinscription_activation_height(chain);

    if below_activation_height {
      !vindicated_set
    } else {
      !vindicated_set || matches!(pre_jubilant_curse_reason, Some(Curse::Reinscription))
    }
  }
}
