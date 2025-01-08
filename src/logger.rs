use anyhow::{anyhow, Context};
use log4rs::{
  append::{
    console::ConsoleAppender,
    rolling_file::{
      policy::compound::{
        roll::fixed_window::FixedWindowRoller, trigger::size::SizeTrigger, CompoundPolicy,
      },
      RollingFileAppender,
    },
  },
  config::{Appender, Logger, Root},
  encode::pattern::PatternEncoder,
  Config,
};
use serde::{Deserialize, Serialize};
use std::{
  fmt::{self, Display, Formatter},
  fs,
  path::Path,
  str::FromStr,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct LogLevel(pub log::LevelFilter);

impl Default for LogLevel {
  fn default() -> Self {
    Self(log::LevelFilter::Error)
  }
}

impl Display for LogLevel {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    Display::fmt(&self.0, f)
  }
}

impl FromStr for LogLevel {
  type Err = <log::LevelFilter as FromStr>::Err;
  fn from_str(level: &str) -> Result<Self, Self::Err> {
    Ok(Self(log::LevelFilter::from_str(level)?))
  }
}

pub fn init<P: AsRef<Path>>(level: LogLevel, log_dir: P) -> anyhow::Result<log4rs::Handle> {
  fs::create_dir_all(&log_dir)?;
  let log_file = log_dir.as_ref().join("ord.log");

  let stdout = ConsoleAppender::builder()
    .encoder(Box::new(PatternEncoder::new(
      "{d(%Y-%m-%d %H:%M:%S.%3f %Z)} {l} [{f}:{L}] {m}{n}",
    )))
    .build();

  // using default encoder for now, change it as needed.
  let encoder = PatternEncoder::default();
  let trigger = SizeTrigger::new(1024 * 1024 * 200);
  let roller = FixedWindowRoller::builder()
    .build(
      log_dir
        .as_ref()
        .join("ord-{}.log.gz")
        .to_string_lossy()
        .as_ref(),
      50,
    )
    .map_err(|e| anyhow!("Failed to build FixedWindowRoller: {}", e))?;
  let policy = CompoundPolicy::new(Box::new(trigger), Box::new(roller));
  let rolling_file = RollingFileAppender::builder()
    .append(true)
    .encoder(Box::new(encoder))
    .build(&log_file, Box::new(policy))
    .map_err(|e| anyhow!("Failed to create rolling file {}", e))?;

  let cfg = Config::builder()
    .appender(Appender::builder().build("stdout", Box::new(stdout)))
    .appender(Appender::builder().build("rfile", Box::new(rolling_file)))
    .logger(Logger::builder().build("mio", log::LevelFilter::Error))
    .build(
      Root::builder()
        .appender("stdout")
        .appender("rfile")
        .build(level.0),
    )
    .map_err(|e| anyhow!("Failed to build log4rs configuration: {}", e))?;

  log4rs::init_config(cfg).map_err(|e| anyhow!("Failed to init log4rs configuration: {}", e))
}
