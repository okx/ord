use super::*;

#[derive(Boilerplate)]
pub(crate) struct ContactHtml {}

impl PageContent for ContactHtml {
  fn title(&self) -> String {
    "Ordinals - Contact".to_string()
  }
}