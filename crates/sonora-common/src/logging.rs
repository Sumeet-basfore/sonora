use tracing::level_filters::LevelFilter;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Logging configuration parameters.
#[derive(Debug, Clone)]
pub struct LogConfig {
    pub default_level: LevelFilter,
    pub enable_ansi: bool,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            default_level: LevelFilter::INFO,
            enable_ansi: true,
        }
    }
}

/// Initialize tracing/logging for Sonora applications and services.
/// Safe to call multiple times (e.g. in tests); subsequent calls will be ignored.
pub fn init_logging(config: &LogConfig) {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        EnvFilter::new(format!(
            "sonora={level},sonora_core={level},sonora_audio={level},sonora_dsp={level},sonora_library={level},sonora_lyrics={level},sonora_common={level}",
            level = config.default_level
        ))
    });

    let fmt_layer = fmt::layer()
        .with_ansi(config.enable_ansi)
        .with_target(true)
        .with_thread_ids(true)
        .with_file(false)
        .with_line_number(false);

    let _ = tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .try_init();
}
