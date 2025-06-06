use std::{fs::read_to_string, process::Command};

use clap::Parser;
use serde::Deserialize;

#[derive(Debug, Parser)]
struct Cli {
  config: String,
}

#[derive(Debug, Deserialize)]
struct Config {
  target: String,
  working_dir: Option<String>,
  args: Vec<String>,
}

fn main() {
  let cli = Cli::parse();

  let config = read_to_string(cli.config).unwrap();

  let config: Config = toml::from_str(&config).unwrap();

  let mut command = Command::new(config.target);
  command.args(config.args);

  if let Some(working_dir) = config.working_dir {
    command.current_dir(working_dir);
  }

  command.spawn().unwrap();
}
