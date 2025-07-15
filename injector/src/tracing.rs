use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{
  Layer as _, filter::FilterFn, fmt::writer::BoxMakeWriter, layer::SubscriberExt as _,
  util::SubscriberInitExt as _,
};

use tracing_subscriber::EnvFilter;

use shared_types::config::injector::DebugConfig;

pub(crate) const HOOKED_PROCESS_OUTPUT_TARGET: &str = "hooked_process.stdout";

#[must_use]
pub(crate) fn init_tracing(
  debug_config: &DebugConfig,
  exit_once_patched: bool,
) -> Option<WorkerGuard> {
  let (non_blocking, guard) = if !exit_once_patched {
    Some(tracing_appender::non_blocking(std::io::stdout()))
  } else {
    None
  }
  .unzip();

  let env_filter = EnvFilter::builder()
    .with_default_directive(debug_config.tracing_level.into())
    .from_env_lossy();

  let suppress_target_output = debug_config.suppress_target_output;
  let dynamic_filter = FilterFn::new(move |metadata| {
    !suppress_target_output || metadata.target() != HOOKED_PROCESS_OUTPUT_TARGET
  });

  let stdout_layer = tracing_subscriber::fmt::layer()
    .map_writer(|w| {
      if let Some(non_blocking) = non_blocking {
        BoxMakeWriter::new(non_blocking)
      } else {
        BoxMakeWriter::new(w)
      }
    })
    .with_filter(env_filter)
    .with_filter(dynamic_filter);
  tracing_subscriber::registry().with(stdout_layer).init();

  guard
}
