use super::*;

impl BRC20ExecutionMessage {
  pub(super) fn execute_inscribe_prog_deploy(
    &self,
    context: &mut TableContext,
  ) -> Result<BRC20Receipt, ExecutionError> {
    let BRC20Operation::InscribeProgDeploy {
      deploy,
      inscription_byte_length,
    } = &self.operation
    else {
      unreachable!()
    };

    context.insert_brc20_prog_deploy_asset(
      self.new_satpoint,
      entry::BRC20ProgDeploy {
        data: deploy.data.clone(),
        base64_data: deploy.base64_data.clone(),
        inscription_id: self.inscription_id.clone(),
        inscription_byte_length: *inscription_byte_length,
      },
    )?;

    let event = BRC20Event::InscribeProgDeploy(InscribeProgDeployEvent {
      data: deploy.data.clone(),
      base64_data: deploy.base64_data.clone(),
    });

    Ok(BRC20Receipt {
      inscription_id: self.inscription_id.clone(),
      sequence_number: self.sequence_number,
      inscription_number: self.inscription_number,
      old_satpoint: self.old_satpoint,
      new_satpoint: self.new_satpoint,
      op_type: BRC20OpType::InscribeProgDeploy,
      sender: self.sender.clone(),
      receiver: self.receiver.clone().unwrap_or(self.sender.clone()),
      result: Ok(event),
      prog_tx_count: 0,
    })
  }
}
