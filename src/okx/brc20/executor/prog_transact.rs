use {
  super::*,
  brc20_prog::types::{Base64Bytes, RawBytes},
};

impl BRC20ExecutionMessage {
  pub(super) fn execute_prog_transact(
    &self,
    brc20_prog_client: &Brc20ProgClient,
    block_timestamp: u64,
    block_hash: &BlockHash,
    tx_idx: u64,
    evm_version_prague: bool,
  ) -> Result<BRC20Receipt, ExecutionError> {
    let BRC20Operation::ProgTransact {
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

    // TODO: not sure if it should be reversed?
    let op_return_tx_id = if evm_version_prague {
      self.txid.to_evm_hash()
    } else {
      Txid::all_zeros().to_evm_hash()
    };

    let prog_tx_count = brc20_prog_client
      .brc20_transact(
        data.clone().map(RawBytes::new),
        base64_data.clone().map(Base64Bytes::new),
        block_timestamp,
        block_hash.to_evm_hash(),
        tx_idx,
        self.inscription_id.to_string(),
        *inscription_byte_length,
        op_return_tx_id,
      )
      .expect("Check your BRC2.0 server")
      .len() as u64;

    Ok(BRC20Receipt {
      inscription_id: self.inscription_id.clone(),
      sequence_number: self.sequence_number,
      inscription_number: self.inscription_number,
      old_satpoint: self.old_satpoint,
      new_satpoint: self.new_satpoint,
      op_type: BRC20OpType::ProgTransact,
      sender: self.sender.clone(),
      receiver: self.receiver.clone().unwrap_or(self.sender.clone()),
      result: Ok(BRC20Event::ProgTransact(ProgTransactEvent {
        data: data.clone(),
        base64_data: base64_data.clone(),
        inscription_byte_length: *inscription_byte_length as u32,
        op_return_tx_id: if evm_version_prague {
          Some(self.txid.to_string())
        } else {
          None
        },
      })),
      prog_tx_count,
    })
  }
}
