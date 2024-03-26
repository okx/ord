use crate::Chain;

pub struct HardForks;

impl HardForks {
  /// Proposed block activation height for issuance and burn enhancements.
  /// Proposal content: https://l1f.discourse.group/t/brc-20-proposal-for-issuance-and-burn-enhancements-brc20-ip-1/621
  pub fn self_issuance_activation_height(chain: Chain) -> u32 {
    match chain {
      Chain::Mainnet => 837090,  // decided by community
      Chain::Testnet => 2413343, // decided by the ourselves
      Chain::Regtest => 0,
      Chain::Signet => 0,
    }
  }
}
