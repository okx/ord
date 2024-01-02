use crate::inscription_id::InscriptionId;
use crate::okx::protocol::zeroindexer::zerodata::ZeroData;
use crate::Inscription;
use redb::ReadableTable;
use std::fmt::{Debug, Display};

pub trait ZeroIndexerReader {
  type Error: Debug + Display;
  fn get_inscription(
    &self,
    inscription_id: &InscriptionId,
  ) -> crate::Result<Option<Inscription>, Self::Error>;

  fn get_zero_indexer_txs(&self, height: u64) -> crate::Result<Option<ZeroData>, Self::Error>;
}

pub trait ZeroIndexerReaderWriter: ZeroIndexerReader {
  fn insert_inscription(
    &mut self,
    inscription_id: &InscriptionId,
    inscription: &Inscription,
  ) -> crate::Result<(), Self::Error>;

  fn remove_inscription(
    &mut self,
    inscription_id: &InscriptionId,
  ) -> crate::Result<(), Self::Error>;

  fn insert_zero_indexer_txs(
    &mut self,
    height: u64,
    data: &ZeroData,
  ) -> crate::Result<(), Self::Error>;
}

pub fn get_zero_indexer_txs<T>(table: &T, height: u64) -> crate::Result<Option<ZeroData>>
where
  T: ReadableTable<u64, &'static [u8]>,
{
  Ok(
    table
      .get(height)?
      .map(|v| bincode::deserialize::<ZeroData>(v.value()).unwrap()),
  )
}
