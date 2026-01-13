use {
  super::*,
  brc20_prog::types::{Base64Bytes, RawBytes},
};

impl BRC20ExecutionMessage {
  pub(super) fn execute_prog_deploy(
    &self,
    brc20_prog_client: &Brc20ProgClient,
    block_timestamp: u64,
    block_hash: &BlockHash,
    prog_tx_idx: &mut u64,
    evm_version_prague: bool,
  ) -> Result<BRC20Receipt, ExecutionError> {
    let BRC20Operation::ProgDeploy {
      data,
      base64_data,
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

    let op_return_tx_id = evm_version_prague
      .then_some(self.txid)
      .unwrap_or(Txid::all_zeros());

    brc20_prog_client.brc20_deploy(
      hex::encode(self.sender.to_script_bytes()),
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
      op_type: BRC20OpType::ProgDeploy,
      sender: self.sender.clone(),
      receiver: self.receiver.clone().unwrap_or(self.sender.clone()),
      result: Ok(BRC20Event::ProgDeploy(ProgDeployEvent {
        data: data.clone(),
        base64_data: base64_data.clone(),
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
