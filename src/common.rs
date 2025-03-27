use tracing::Level;

pub fn init_logger() {
    let ts = tracing_subscriber::fmt();
    #[cfg(test)]
    let ts = ts.with_max_level(Level::INFO).pretty();
    ts.with_writer(std::io::stderr).init();
}
