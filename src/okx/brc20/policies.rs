use {
  crate::{index::Curse, okx::UtxoAddress, Chain},
  bitcoin::Script,
  ordinals::Charm,
};

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

  /// Proposed block activation height for BRC-20 Prog phase one.
  /// Proposal content: https://github.com/brc20-devs/brc20-proposals/blob/main/bp08-module-swap-refund/proposal.md
  pub fn brc20_swap_refund_activation_height(chain: &Chain) -> u32 {
    match chain {
      Chain::Mainnet => 932888, // decided by community
      Chain::Testnet => 0,
      Chain::Regtest => 0,
      Chain::Signet => 283888,
      Chain::Testnet4 => 0,
    }
  }

  pub fn brc20_swap_refund_addresses(chain: &Chain) -> Option<(UtxoAddress, UtxoAddress)> {
    match chain {
      Chain::Mainnet => Some((
        UtxoAddress::from_script(
          Script::from_bytes(
            hex::decode("6a208cbc2ac9896cac98d304aa42f43b98208ce8ae31c25e48d84ee852834a1a8066")
              .unwrap()
              .as_slice(),
          ),
          &Chain::Mainnet,
        ),
        UtxoAddress::from_str(
          "bc1qrj03km5h9ag24cpynyn3l5tvny9cyd0x0le42u",
          bitcoin::Network::Bitcoin,
        )
        .unwrap(),
      )),
      Chain::Testnet => None,
      Chain::Regtest => None,
      Chain::Signet => Some((
        UtxoAddress::from_script(
          Script::from_bytes(
            hex::decode("6a2031152ea2364a6dbaab9013be8d656a34256d7e89f0c30714bbb04a9d85e63e5b")
              .unwrap()
              .as_slice(),
          ),
          &Chain::Signet,
        ),
        UtxoAddress::from_str(
          "tb1qkrewl9zclku2qngth52eezdyrwmjpcspttdypa",
          bitcoin::Network::Signet,
        )
        .unwrap(),
      )),
      Chain::Testnet4 => None,
    }
  }

  /// Proposed block activation height for BRC-20 Prog phase one.
  ///
  /// This height enables the programmable module feature for 6-byte tickers. 4 and 5-byte tickers
  /// will not have access to the programmable module until the second phase activation height.
  ///
  /// Proposal content: https://github.com/bestinslot-xyz/brc20-proposals/blob/main/000-programmable-module/index.md
  pub fn brc20_prog_activation_height(chain: &Chain) -> u32 {
    match chain {
      Chain::Mainnet => 912690, // decided by community
      Chain::Testnet => 0,      // decided by okx team
      Chain::Regtest => 0,
      Chain::Signet => 230000,
      Chain::Testnet4 => 0,
    }
  }

  /// Proposed block activation height for EVM version Prague.
  ///
  /// This height enables the use of the transaction ID in the OP_RETURN field for BRC-20 Prog operations.
  /// Proposal content: https://github.com/bestinslot-xyz/brc20-proposals/blob/main/003-prog-evm-upgrade/index.md
  pub fn brc20_prog_prague_activation_height(chain: &Chain) -> u32 {
    match chain {
      Chain::Mainnet => 923369, // decided by community
      Chain::Testnet => 0,      // decided by okx team
      Chain::Regtest => 0,
      Chain::Signet => 275000,
      Chain::Testnet4 => 0,
    }
  }

  /// Proposed block activation height for BRC-20 Prog phase two.
  ///
  /// This height enables the programmable module feature for all tickers, including 4 and 5-byte tickers.
  ///
  /// Proposal content: https://github.com/bestinslot-xyz/brc20-proposals/blob/main/000-programmable-module/index.md
  pub fn brc20_prog_all_tickers_activation_height(chain: &Chain) -> u32 {
    match chain {
      Chain::Mainnet => 934888, // decided by community
      Chain::Testnet => 0,      // decided by okx team
      Chain::Regtest => 0,
      Chain::Signet => 230000,
      Chain::Testnet4 => 0,
    }
  }

  /// Proposed block activation height for pre-deploy feature.
  /// It is 10 blocks earlier than the 6-byte deployment activation height to allow users
  /// to pre-deploy their desired tickers before the actual deployment.
  /// Proposal content: https://github.com/bestinslot-xyz/brc20-proposals/tree/main/001-6-byte-namespace/index.md
  pub fn predeploy_activation_height(chain: &Chain) -> u32 {
    Self::six_byte_deploy_activation_height(chain) - 10
  }

  /// Proposed block activation height for 6-byte deployment feature.
  /// Proposal content: https://github.com/bestinslot-xyz/brc20-proposals/tree/main/001-6-byte-namespace/index.md
  pub fn six_byte_deploy_activation_height(chain: &Chain) -> u32 {
    Self::brc20_prog_activation_height(chain)
  }

  pub fn draft_reinscription_activation_height(chain: &Chain) -> u32 {
    match chain {
      Chain::Mainnet => u32::MAX, // todo: not set yet
      Chain::Testnet => u32::MAX,
      Chain::Regtest => u32::MAX,
      Chain::Signet => u32::MAX,
      Chain::Testnet4 => u32::MAX,
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
