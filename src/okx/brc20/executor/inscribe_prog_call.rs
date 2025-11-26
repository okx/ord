use super::*;

impl BRC20ExecutionMessage {
  pub(super) fn execute_inscribe_prog_call(
    &self,
    context: &mut TableContext,
  ) -> Result<BRC20Receipt, ExecutionError> {
    let BRC20Operation::InscribeProgCall {
      call,
      inscription_byte_length,
    } = &self.operation
    else {
      unreachable!()
    };

    if call.data.is_some() && call.base64_data.is_some() {
      return Err(ExecutionError::ExecutionFailed(
        BRC20Error::BRC20ProgDataConflict,
      ));
    };

    if call.data.is_none() && call.base64_data.is_none() {
      return Err(ExecutionError::ExecutionFailed(
        BRC20Error::BRC20ProgDataMissing,
      ));
    };

    context.insert_brc20_prog_call_asset(
      self.new_satpoint,
      entry::BRC20ProgCall {
        contract_address: call.contract_address.clone(),
        contract_inscription_id: call.inscription_id.clone(),
        data: call.data.clone(),
        base64_data: call.base64_data.clone(),
        inscription_byte_length: *inscription_byte_length,
        inscription_id: self.inscription_id.clone(),
      },
    )?;

    let event = BRC20Event::InscribeProgCall(InscribeProgCallEvent {
      inscription_id: call.inscription_id.clone(),
      contract_address: call.contract_address.clone(),
      data: call.data.clone(),
      base64_data: call.base64_data.clone(),
    });

    Ok(BRC20Receipt {
      inscription_id: self.inscription_id.clone(),
      sequence_number: self.sequence_number,
      inscription_number: self.inscription_number,
      old_satpoint: self.old_satpoint,
      new_satpoint: self.new_satpoint,
      op_type: BRC20OpType::InscribeProgCall,
      sender: self.sender.clone(),
      receiver: self.receiver.clone().unwrap_or(self.sender.clone()),
      result: Ok(event),
      prog_tx_count: 0,
    })
  }
}
