use clap::Parser;
use shared_types::config::injector::InjectorConfig;

use injector::config::Cli;

#[tokio::main]
async fn main() {
  let cli = Cli::parse();
  let config = InjectorConfig::from(cli);

  injector::inject(config).await
}
