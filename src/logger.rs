#[cfg(not(feature = "telemetry"))]
pub fn init(_service_name: &'static str) {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_writer(std::io::stderr)
        .init();
}

#[cfg(feature = "telemetry")]
pub fn init(service_name: &'static str) {
    crate::telemetry::init(service_name);
}
