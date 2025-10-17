use crate::okx::{
  brc20::{
    entry::BRC20Predeploy,
    event::{BRC20Event, BRC20OpType, PredeployEvent},
    executor::ExecutionError,
    BRC20ExecutionMessage, BRC20Operation, BRC20Receipt,
  },
  context::TableContext,
};

impl BRC20ExecutionMessage {
  pub(super) fn execute_predeploy(
    &self,
    context: &mut TableContext,
    height: u32,
  ) -> Result<BRC20Receipt, ExecutionError> {
    let BRC20Operation::Predeploy(predeploy) = &self.operation else {
      unreachable!()
    };

    // Insert the new predeploy record.
    let predeploy_record = BRC20Predeploy {
      hash: predeploy.hash,
      predeployer: self.receiver.clone().unwrap(),
      block_height: height,
    };
    context.insert_brc20_predeploy(&self.inscription_id, predeploy_record)?;

    Ok(BRC20Receipt {
      inscription_id: self.inscription_id,
      sequence_number: self.sequence_number,
      inscription_number: self.inscription_number,
      old_satpoint: self.old_satpoint,
      new_satpoint: self.new_satpoint,
      sender: self.sender.clone(),
      receiver: self.receiver.clone().unwrap(),
      op_type: BRC20OpType::Predeploy,
      result: Ok(BRC20Event::Predeploy(PredeployEvent {
        hash: predeploy.hash,
        predeployer: self.receiver.clone().unwrap(),
        block_height: height,
      })),
    })
  }
}
