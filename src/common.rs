use tracing::Level;

pub fn init_logger() {
    let ts = tracing_subscriber::fmt();
    #[cfg(test)]
    let ts = ts.with_max_level(Level::INFO).pretty();
    #[cfg(not(test))]
    let ts = ts.with_max_level(Level::DEBUG);
    ts.with_writer(std::io::stderr).init();
}
