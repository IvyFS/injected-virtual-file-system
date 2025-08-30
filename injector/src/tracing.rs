use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{
  Layer as _, filter::FilterFn, layer::SubscriberExt as _, util::SubscriberInitExt as _,
};

use tracing_subscriber::EnvFilter;

use shared_types::config::injector::DebugConfig;

pub(crate) const HOOKED_PROCESS_OUTPUT_TARGET: &str = "hooked_process.stdout";
pub(crate) const INJECTOR_PROFILING_TARGET: &str = "injector.profiling";

#[must_use]
pub(crate) fn init_tracing(debug_config: &DebugConfig) -> WorkerGuard {
  let (non_blocking, guard) = tracing_appender::non_blocking(std::io::stdout());

  let env_filter = EnvFilter::builder()
    .with_default_directive(debug_config.tracing_level.into())
    .from_env_lossy();

  let suppress_target_output = debug_config.suppress_target_output;
  let enable_profiling = debug_config.profiling;
  let dynamic_filter = FilterFn::new(move |metadata| match metadata.target() {
    HOOKED_PROCESS_OUTPUT_TARGET => !suppress_target_output,
    INJECTOR_PROFILING_TARGET => enable_profiling,
    _ => true,
  });

  let stdout_layer = tracing_subscriber::fmt::layer()
    .with_writer(non_blocking)
    .with_filter(env_filter)
    .with_filter(dynamic_filter);
  tracing_subscriber::registry().with(stdout_layer).init();

  guard
}
