pub(crate) mod brc20;
pub(crate) mod context;
pub(crate) mod execute_manager;
pub(crate) mod message;
pub(crate) mod ord;
pub(crate) mod protocol_manager;
pub(crate) mod resolve_manager;

pub use self::protocol_manager::ProtocolManager;

use {
  self::{execute_manager::CallManager, message::Message, resolve_manager::MsgResolveManager},
  crate::{Chain, Options},
};

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct ChainContext {
  pub chain: Chain,
  pub blockheight: u32,
  pub blocktime: u32,
}
#[derive(Debug, Clone)]
pub struct ProtocolConfig {
  first_inscription_height: u32,
  first_brc20_height: Option<u32>,
  enable_ord_receipts: bool,
  enable_index_bitmap: bool,
  enable_index_domain: bool,
  btc_domain_list: Vec<String>,
}

impl ProtocolConfig {
  pub(crate) fn new_with_options(options: &Options) -> Self {
    Self {
      first_inscription_height: options.first_inscription_height(),
      first_brc20_height: if options.enable_index_brc20 {
        Some(options.first_brc20_height())
      } else {
        None
      },
      enable_ord_receipts: options.enable_save_ord_receipts,
      enable_index_bitmap: options.enable_index_bitmap,
      enable_index_domain: options.enable_index_domain,
      btc_domain_list: options.btc_domain_list.clone(),
    }
  }
}
