use super::*;

impl BRC20ExecutionMessage {
  pub(super) fn execute_inscribe_prog_transact(
    &self,
    context: &mut TableContext,
  ) -> Result<BRC20Receipt, ExecutionError> {
    let BRC20Operation::InscribeProgTransact {
      transact,
      inscription_byte_length,
    } = &self.operation
    else {
      unreachable!()
    };

    if transact.data.is_some() && transact.base64_data.is_some() {
      return Err(ExecutionError::ExecutionFailed(
        BRC20Error::BRC20ProgDataConflict,
      ));
    };

    if transact.data.is_none() && transact.base64_data.is_none() {
      return Err(ExecutionError::ExecutionFailed(
        BRC20Error::BRC20ProgDataMissing,
      ));
    };

    context.insert_brc20_prog_transact_asset(
      self.new_satpoint,
      entry::BRC20ProgTransact {
        data: transact.data.clone(),
        base64_data: transact.base64_data.clone(),
        inscription_id: self.inscription_id.clone(),
        inscription_byte_length: *inscription_byte_length,
      },
    )?;

    let event = BRC20Event::InscribeProgTransact(InscribeProgTransactEvent {
      data: transact.data.clone(),
      base64_data: transact.base64_data.clone(),
    });

    Ok(BRC20Receipt {
      inscription_id: self.inscription_id.clone(),
      sequence_number: self.sequence_number,
      inscription_number: self.inscription_number,
      old_satpoint: self.old_satpoint,
      new_satpoint: self.new_satpoint,
      op_type: BRC20OpType::InscribeProgTransact,
      sender: self.sender.clone(),
      receiver: self.receiver.clone().unwrap_or(self.sender.clone()),
      result: Ok(event),
    })
  }
}
