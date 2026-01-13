use log::{log_enabled, Level};

use crate::okx::brc20::{error::BRC20Error, event::BRC20Event, BRC20Receipt};

pub(crate) const EVENT_SEPARATOR: &str = "|";

pub struct BRC20BlockEventHash {
  events: Vec<String>,
}

impl BRC20BlockEventHash {
  pub fn new() -> Self {
    Self { events: Vec::new() }
  }

  pub fn add_receipt(&mut self, receipt: BRC20Receipt) {
    match receipt.result {
      Ok(BRC20Event::Predeploy(predeploy_event)) => {
        self.events.push(format!(
          "predeploy-inscribe;{};{};{};{}",
          receipt.inscription_id,
          hex::encode(predeploy_event.predeployer.to_script_bytes()),
          predeploy_event.hash,
          predeploy_event.block_height
        ));
      }
      Ok(BRC20Event::Deploy(deploy_event)) => {
        self.events.push(format!(
          "deploy-inscribe;{};{};{};{};{};{};{};{}",
          receipt.inscription_id,
          hex::encode(receipt.receiver.to_script_bytes()),
          deploy_event.ticker.to_lowercase(),
          deploy_event.ticker,
          number_string_with_full_decimals(deploy_event.total_supply, deploy_event.decimals),
          deploy_event.decimals,
          number_string_with_full_decimals(deploy_event.max_mint_limit, deploy_event.decimals),
          deploy_event.self_minted,
        ));
      }
      Ok(BRC20Event::Mint(mint_event)) => {
        self.events.push(format!(
          "mint-inscribe;{};{};{};{};{};{}",
          receipt.inscription_id,
          hex::encode(receipt.receiver.to_script_bytes()),
          mint_event.ticker.to_lowercase(),
          mint_event.original_ticker,
          number_string_with_full_decimals(mint_event.amount, mint_event.decimals),
          mint_event.parent_id.unwrap_or_default()
        ));
      }
      Ok(BRC20Event::InscribeTransfer(inscribe_transfer_event)) => {
        self.events.push(format!(
          "transfer-inscribe;{};{};{};{};{}",
          receipt.inscription_id,
          hex::encode(receipt.receiver.to_script_bytes()),
          inscribe_transfer_event.ticker.to_lowercase(),
          inscribe_transfer_event.original_ticker,
          number_string_with_full_decimals(
            inscribe_transfer_event.amount,
            inscribe_transfer_event.decimals
          ),
        ));
      }
      Ok(BRC20Event::Transfer(transfer_event)) => {
        self.events.push(format!(
          "transfer-transfer;{};{};{};{};{};{}",
          receipt.inscription_id,
          hex::encode(receipt.sender.to_script_bytes()),
          if transfer_event.send_to_coinbase {
            "".to_string()
          } else {
            hex::encode(receipt.receiver.to_script_bytes())
          },
          transfer_event.ticker.to_lowercase(),
          transfer_event.original_ticker,
          number_string_with_full_decimals(transfer_event.amount, transfer_event.decimals)
        ));
      }
      Ok(BRC20Event::InscribeProgDeploy(inscribe_prog_deploy_event)) => self.events.push(format!(
        "brc20prog-deploy-inscribe;{};{};{};{}",
        receipt.inscription_id,
        hex::encode(receipt.receiver.to_script_bytes()),
        inscribe_prog_deploy_event.data.unwrap_or_default(),
        inscribe_prog_deploy_event.base64_data.unwrap_or_default(),
      )),
      Ok(BRC20Event::ProgDeploy(prog_deploy_event)) => match prog_deploy_event.op_return_tx_id {
        Some(tx_id) => self.events.push(format!(
          "brc20prog-deploy-transfer;{};{};{};{};{};{};{}",
          receipt.inscription_id,
          hex::encode(receipt.sender.to_script_bytes()),
          hex::encode(receipt.receiver.to_script_bytes()),
          prog_deploy_event.data.unwrap_or_default(),
          prog_deploy_event.base64_data.unwrap_or_default(),
          prog_deploy_event.inscription_byte_length,
          tx_id,
        )),
        None => self.events.push(format!(
          "brc20prog-deploy-transfer;{};{};{};{};{};{}",
          receipt.inscription_id,
          hex::encode(receipt.sender.to_script_bytes()),
          hex::encode(receipt.receiver.to_script_bytes()),
          prog_deploy_event.data.unwrap_or_default(),
          prog_deploy_event.base64_data.unwrap_or_default(),
          prog_deploy_event.inscription_byte_length
        )),
      },
      Ok(BRC20Event::InscribeProgCall(inscribe_prog_call_event)) => self.events.push(format!(
        "brc20prog-call-inscribe;{};{};{};{};{};{}",
        receipt.inscription_id,
        hex::encode(receipt.receiver.to_script_bytes()),
        inscribe_prog_call_event
          .contract_address
          .unwrap_or_default(),
        inscribe_prog_call_event.inscription_id.unwrap_or_default(),
        inscribe_prog_call_event.data.unwrap_or_default(),
        inscribe_prog_call_event.base64_data.unwrap_or_default(),
      )),
      Ok(BRC20Event::ProgCall(prog_call_event)) => match prog_call_event.op_return_tx_id {
        Some(tx_id) => self.events.push(format!(
          "brc20prog-call-transfer;{};{};{};{};{};{};{};{};{}",
          receipt.inscription_id,
          hex::encode(receipt.sender.to_script_bytes()),
          hex::encode(receipt.receiver.to_script_bytes()),
          prog_call_event.contract_address.unwrap_or_default(),
          prog_call_event.inscription_id.unwrap_or_default(),
          prog_call_event.data.unwrap_or_default(),
          prog_call_event.base64_data.unwrap_or_default(),
          prog_call_event.inscription_byte_length,
          tx_id,
        )),
        None => self.events.push(format!(
          "brc20prog-call-transfer;{};{};{};{};{};{};{};{}",
          receipt.inscription_id,
          hex::encode(receipt.sender.to_script_bytes()),
          hex::encode(receipt.receiver.to_script_bytes()),
          prog_call_event.contract_address.unwrap_or_default(),
          prog_call_event.inscription_id.unwrap_or_default(),
          prog_call_event.data.unwrap_or_default(),
          prog_call_event.base64_data.unwrap_or_default(),
          prog_call_event.inscription_byte_length,
        )),
      },
      Ok(BRC20Event::InscribeProgTransact(inscribe_prog_transact_event)) => {
        self.events.push(format!(
          "brc20prog-transact-inscribe;{};{};{};{}",
          receipt.inscription_id,
          hex::encode(receipt.receiver.to_script_bytes()),
          inscribe_prog_transact_event.data.unwrap_or_default(),
          inscribe_prog_transact_event.base64_data.unwrap_or_default(),
        ))
      }
      Ok(BRC20Event::ProgTransact(prog_transact_event)) => {
        match prog_transact_event.op_return_tx_id {
          Some(tx_id) => self.events.push(format!(
            "brc20prog-transact-transfer;{};{};{};{};{};{};{}",
            receipt.inscription_id,
            hex::encode(receipt.sender.to_script_bytes()),
            hex::encode(receipt.receiver.to_script_bytes()),
            prog_transact_event.data.unwrap_or_default(),
            prog_transact_event.base64_data.unwrap_or_default(),
            prog_transact_event.inscription_byte_length,
            tx_id,
          )),
          None => self.events.push(format!(
            "brc20prog-transact-transfer;{};{};{};{};{};{}",
            receipt.inscription_id,
            hex::encode(receipt.sender.to_script_bytes()),
            hex::encode(receipt.receiver.to_script_bytes()),
            prog_transact_event.data.unwrap_or_default(),
            prog_transact_event.base64_data.unwrap_or_default(),
            prog_transact_event.inscription_byte_length,
          )),
        }
      }
      Ok(BRC20Event::InscribeWithdraw(inscribe_withdraw)) => {
        self.events.push(format!(
          "brc20prog-withdraw-inscribe;{};{};{};{};{}",
          receipt.inscription_id,
          hex::encode(receipt.receiver.to_script_bytes()),
          inscribe_withdraw.ticker.to_lowercase(),
          inscribe_withdraw.original_ticker,
          number_string_with_full_decimals(inscribe_withdraw.amount, inscribe_withdraw.decimals),
        ));
      }
      Ok(BRC20Event::Withdraw(withdraw_event))
      | Err(BRC20Error::WithdrawExecutionFailed(withdraw_event))
      | Err(BRC20Error::InvalidBRC20WithdrawReceiverAddress(withdraw_event)) => {
        self.events.push(format!(
          "brc20prog-withdraw-transfer;{};{};{};{};{};{}",
          receipt.inscription_id,
          hex::encode(receipt.sender.to_script_bytes()),
          hex::encode(receipt.receiver.to_script_bytes()),
          withdraw_event.ticker.to_lowercase(),
          withdraw_event.original_ticker,
          number_string_with_full_decimals(withdraw_event.amount, withdraw_event.decimals),
        ));
      }
      _ => { /* Ignore other event types */ }
    }
  }

  pub fn get_block_event_hash(&self) -> String {
    let concatenated = self.events.join(EVENT_SEPARATOR);
    let hash = sha256::digest(&concatenated);
    // Debug is doing much more expensive operations, so guard it
    if log_enabled!(Level::Debug) && !concatenated.is_empty() {
      static INLINE_SEPARATOR: &str = ";";
      let shortened_concatenated = shorten_parts(&concatenated, INLINE_SEPARATOR, 70);
      log::debug!(
        "BRC20 Block Event Concatenated String: {}",
        shortened_concatenated
      );
      log::debug!("BRC20 Block Event Hash: {}", hash);
    }
    hash
  }
}

fn number_string_with_full_decimals(number: u128, decimals: u8) -> String {
  let mut number_str = number.to_string();

  if number_str.len() < decimals as usize {
    let leading_zeros = "0".repeat(decimals as usize - number_str.len());
    number_str = format!("{}{}", leading_zeros, number_str);
  }

  if decimals != 0 {
    let insert_index = number_str.len() - decimals as usize;
    number_str.insert(insert_index, '.');

    if insert_index == 0 {
      number_str.insert(0, '0');
    }
  }

  number_str
}

pub fn shorten_parts(input: &str, inline_separator: &str, max_len: usize) -> String {
  // Some hashes can be very long, so shorten them with .. in the middle for logging
  // Hashes are between ; and ; so we can safely do this
  // If it's longer than 70 characters, we shorten it to 32..32 characters
  // 70 is chosen because inscription ids are 66 characters long and we add some buffer
  let events: Vec<&str> = input.split(EVENT_SEPARATOR).collect();
  let mut shortened_events: Vec<String> = Vec::new();
  for event in events {
    let inner_parts = event.split(inline_separator).collect::<Vec<&str>>();
    let mut shortened_part = String::new();
    for inner_part in inner_parts {
      if inner_part.len() > max_len {
        let shortened = format!(
          "{}..{}",
          &inner_part[..32],
          &inner_part[inner_part.len() - 32..]
        );
        shortened_part.push_str(&shortened);
      } else {
        shortened_part.push_str(inner_part);
      }
      shortened_part.push_str(inline_separator);
    }
    // Remove last separator
    shortened_part.pop();
    shortened_events.push(shortened_part);
  }
  shortened_events.join(EVENT_SEPARATOR)
}
