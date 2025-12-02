use {
  super::*,
  brc20_prog::types::{AddressED, Base64Bytes, RawBytes},
};

impl BRC20ExecutionMessage {
  pub(super) fn execute_prog_call(
    &self,
    brc20_prog_client: &Brc20ProgClient,
    block_timestamp: u64,
    block_hash: &BlockHash,
    prog_tx_idx: &mut u64,
    evm_version_prague: bool,
  ) -> Result<BRC20Receipt, ExecutionError> {
    let BRC20Operation::ProgCall {
      data,
      base64_data,
      contract_address,
      contract_inscription_id,
      inscription_byte_length,
    } = &self.operation
    else {
      unreachable!()
    };

    if let Some(receiver) = self.receiver.clone() {
      if !receiver.op_return_prog() {
        return Err(ExecutionError::ExecutionFailed(
          BRC20Error::InvalidBRC20ProgReceiverAddress,
        ));
      }
    }

    let contract_address_ed = contract_address
      .clone()
      .map(|address| AddressED::try_from(address.as_str()).ok())
      .flatten();

    let op_return_tx_id = evm_version_prague
      .then_some(self.txid)
      .unwrap_or(Txid::all_zeros());

    brc20_prog_client.brc20_call(
      hex::encode(self.sender.to_script_bytes()),
      contract_address_ed,
      contract_inscription_id.clone(),
      data.clone().map(RawBytes::new),
      base64_data.clone().map(Base64Bytes::new),
      block_timestamp,
      block_hash.to_b256_ed(),
      *prog_tx_idx,
      self.inscription_id.to_string(),
      *inscription_byte_length,
      op_return_tx_id.to_b256_ed(),
    )?;
    *prog_tx_idx += 1;
    Ok(BRC20Receipt {
      inscription_id: self.inscription_id.clone(),
      sequence_number: self.sequence_number,
      inscription_number: self.inscription_number,
      old_satpoint: self.old_satpoint,
      new_satpoint: self.new_satpoint,
      op_type: BRC20OpType::ProgCall,
      sender: self.sender.clone(),
      receiver: self.receiver.clone().unwrap_or(self.sender.clone()),
      result: Ok(BRC20Event::ProgCall(ProgCallEvent {
        data: data.clone(),
        base64_data: base64_data.clone(),
        contract_address: contract_address.clone(),
        inscription_id: contract_inscription_id.clone(),
        inscription_byte_length: *inscription_byte_length as u32,
        op_return_tx_id: if evm_version_prague {
          Some(self.txid.to_string())
        } else {
          None
        },
      })),
    })
  }
}
