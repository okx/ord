use super::{entry::CollectionType, *};
use once_cell::sync::Lazy;

#[derive(Debug, Clone)]
pub struct BitmapDistrict {
  number: u32,
}

pub trait BitmapMessageExtractor {
  fn extract_bitmap_message(&self) -> Option<BitmapDistrict>;
}

impl BitmapMessageExtractor for Inscription {
  fn extract_bitmap_message(&self) -> Option<BitmapDistrict> {
    let Some(content) = self.body() else {
      return None;
    };

    static RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^(0|[1-9][0-9]*)\.bitmap$").unwrap());

    let content = std::str::from_utf8(content).ok()?;

    let capture = RE.captures(content)?;
    let number_str = capture.get(1)?.as_str();

    let number = number_str.parse().ok()?;

    Some(BitmapDistrict { number })
  }
}

impl BitmapDistrict {
  pub(crate) fn execute(
    &self,
    context: &mut TableContext,
    sequence_number: u32,
    inscription_id: InscriptionId,
    height: u32,
  ) -> Result<()> {
    if self.number > height {
      log::debug!(
        "BitmapDistrict::execute: bitmap height {} is greater than block height {}, inscription_id {}",
        self.number,
        height,
        inscription_id,
      );
      return Ok(());
    }

    if context
      .load_bitmap_block_height_to_sequence_number(self.number)?
      .is_some()
    {
      log::debug!(
        "BitmapDistrict::execute: bitmap for height {} already exists, inscription_id {}",
        self.number,
        inscription_id,
      );
      return Ok(());
    }

    context.insert_bitmap_block_height_to_sequence_number(self.number, sequence_number)?;
    context.insert_sequence_number_to_collection_type(sequence_number, CollectionType::Bitmap)?;
    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn validate_regex() {
    let district = Inscription {
      body: Some(b"0.bitmap".as_slice().into()),
      ..default()
    }
    .extract_bitmap_message()
    .unwrap();
    assert_eq!(district.number, 0);

    let district = Inscription {
      body: Some(b"40.bitmap".as_slice().into()),
      ..default()
    }
    .extract_bitmap_message()
    .unwrap();
    assert_eq!(district.number, 40);
  }

  #[test]
  fn invalidate_regex() {
    assert!(Inscription {
      body: Some(b".bitmap".as_slice().into()),
      ..default()
    }
    .extract_bitmap_message()
    .is_none());

    assert!(Inscription {
      body: Some(b"bitmap".as_slice().into()),
      ..default()
    }
    .extract_bitmap_message()
    .is_none());

    assert!(Inscription {
      body: Some(b"c.bitmap".as_slice().into()),
      ..default()
    }
    .extract_bitmap_message()
    .is_none());

    assert!(Inscription {
      body: Some(b"111".as_slice().into()),
      ..default()
    }
    .extract_bitmap_message()
    .is_none());

    assert!(Inscription {
      body: Some(b"01.bitmap".as_slice().into()),
      ..default()
    }
    .extract_bitmap_message()
    .is_none());
    assert!(Inscription {
      body: Some((u32::MAX.to_string() + "1.bitmap").as_bytes().into()),
      ..default()
    }
    .extract_bitmap_message()
    .is_none());
  }
}
