use clap::{ArgAction, Parser, Subcommand};
use shared_types::config::{
  VirtualFsConfig, injector::DebugConfig, injector::InjectorConfig, injector::TargetConfig,
};

#[derive(Debug, Parser)]
#[command(
  subcommand_negates_reqs = true,
  args_conflicts_with_subcommands = true,
  flatten_help = true,
  disable_help_subcommand = true,
  disable_help_flag = true
)]
pub struct Cli {
  #[command(subcommand)]
  running: Option<Command>,

  #[arg(required = true)]
  config: Option<String>,

  #[arg(long, short, action = ArgAction::Help, global = true, hide = true)]
  help: Option<bool>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
  Existing {
    #[arg(long)]
    pid: u32,
    #[arg(long)]
    virtual_root: String,
    #[arg(long)]
    mount_point: String,
  },
}

impl From<Cli> for InjectorConfig {
  fn from(value: Cli) -> Self {
    let Cli {
      running, config, ..
    } = value;

    match (running, config) {
      (
        Some(Command::Existing {
          pid,
          virtual_root,
          mount_point,
        }),
        _,
      ) => InjectorConfig {
        virtual_filesystem: VirtualFsConfig {
          mount_point: mount_point.into(),
          virtual_root: virtual_root.into(),
        },
        debug: DebugConfig {
          suppress_target_output: false,
          print_hook_logs_to_console: true,
          ..Default::default()
        },
        target: TargetConfig {
          pid: Some(pid),
          ..Default::default()
        },
        instant_shutdown: true,
        return_target_exit_code: false,
      },
      (_, Some(config_path)) => InjectorConfig::parse_or_panic(config_path),
      _ => unreachable!(),
    }
  }
}
