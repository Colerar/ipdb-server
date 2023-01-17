use std::path::PathBuf;

use anyhow::{Context, Result};
use clap_verbosity_flag::LogLevel;
use log::{Level, LevelFilter};
use log4rs::{
  append::console::ConsoleAppender,
  config::{Appender, Root},
  encode::pattern::PatternEncoder,
  Config,
};

pub fn setup(verbosity: LevelFilter, config_path: Option<PathBuf>) -> Result<()> {
  match config_path {
    Some(path) => {
      log4rs::init_file(path, Default::default()).context("Failed to init log4rs config")?;
    }
    None => {
      const PATTERN: &str = "{d(%m-%d %H:%M)} {h({l:.1})} - {h({m})}{n}";
      let stdout = ConsoleAppender::builder()
        .encoder(Box::new(PatternEncoder::new(PATTERN)))
        .build();
      let config = Config::builder()
        .appender(Appender::builder().build("stdout", Box::new(stdout)))
        .build(Root::builder().appender("stdout").build(verbosity))
        .unwrap();
      log4rs::init_config(config).context("Failed to init log4rs config")?;
    }
  };
  Ok(())
}

#[cfg(debug_assertions)]
pub type DefaultLevel = DebugLevel;

#[cfg(not(debug_assertions))]
pub type DefaultLevel = clap_verbosity_flag::InfoLevel;

#[derive(Copy, Clone, Debug, Default)]
pub struct DebugLevel;

impl LogLevel for DebugLevel {
  fn default() -> Option<Level> {
    Some(Level::Debug)
  }
}
