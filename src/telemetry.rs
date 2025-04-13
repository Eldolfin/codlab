use opentelemetry::global;
use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use opentelemetry_otlp::SpanExporter;
use opentelemetry_otlp::{LogExporter, MetricExporter};
use opentelemetry_sdk::trace::SdkTracerProvider;
use opentelemetry_sdk::{logs::SdkLoggerProvider, metrics::SdkMeterProvider, Resource};
use tracing_subscriber::{
    layer::SubscriberExt as _, util::SubscriberInitExt as _, EnvFilter, Layer,
};

pub struct Providers {
    logs: SdkLoggerProvider,
    traces: SdkTracerProvider,
    metrics: SdkMeterProvider,
    is_shutdown: bool,
}

fn init_logs(resource: Resource) -> SdkLoggerProvider {
    let exporter = LogExporter::builder()
        .with_tonic()
        .build()
        .expect("Failed to create log exporter");

    SdkLoggerProvider::builder()
        .with_resource(resource)
        .with_batch_exporter(exporter)
        .build()
}

fn init_traces(resource: Resource) -> SdkTracerProvider {
    let exporter = SpanExporter::builder()
        .with_tonic()
        .build()
        .expect("Failed to create trace exporter");

    SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .with_resource(resource)
        .build()
}

fn init_metrics(resource: Resource) -> SdkMeterProvider {
    let exporter = MetricExporter::builder()
        .with_tonic()
        .build()
        .expect("Failed to create metric exporter");

    SdkMeterProvider::builder()
        .with_periodic_exporter(exporter)
        .with_resource(resource)
        .build()
}

pub fn init(service_name: &'static str) -> Providers {
    let resource = Resource::builder().with_service_name(service_name).build();
    let logger_provider = init_logs(resource.clone());
    let otel_layer = OpenTelemetryTracingBridge::new(&logger_provider);

    // https://github.com/open-telemetry/opentelemetry-rust/blob/7bdd2f4160438b9a4cbf3092057f3e7dc9a6a95f/opentelemetry-otlp/examples/basic-otlp-http/src/main.rs#L75
    let filter_otel = EnvFilter::new("info")
        .add_directive("hyper=off".parse().unwrap())
        .add_directive("tonic=off".parse().unwrap())
        .add_directive("h2=off".parse().unwrap())
        .add_directive("reqwest=off".parse().unwrap());
    let otel_layer = otel_layer.with_filter(filter_otel);

    let filter_fmt = EnvFilter::new("info").add_directive("codlab=debug".parse().unwrap());
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_thread_names(true)
        .with_filter(filter_fmt);

    tracing_subscriber::registry()
        .with(otel_layer)
        .with(fmt_layer)
        .init();

    let tracer_provider = init_traces(resource.clone());
    global::set_tracer_provider(tracer_provider.clone());

    let meter_provider = init_metrics(resource.clone());
    global::set_meter_provider(meter_provider.clone());

    Providers {
        traces: tracer_provider,
        metrics: meter_provider,
        logs: logger_provider,
        is_shutdown: false,
    }
}

impl Providers {
    pub fn shutdown(mut self) -> anyhow::Result<()> {
        // Collect all shutdown errors
        let mut shutdown_errors = Vec::new();
        if let Err(e) = self.traces.shutdown() {
            shutdown_errors.push(format!("tracer provider: {}", e));
        }

        if let Err(e) = self.metrics.shutdown() {
            shutdown_errors.push(format!("meter provider: {}", e));
        }

        if let Err(e) = self.logs.shutdown() {
            shutdown_errors.push(format!("logger provider: {}", e));
        }

        // Return an error if any shutdown failed
        if !shutdown_errors.is_empty() {
            return Err(anyhow::anyhow!(
                "Failed to shutdown providers:\n{}",
                shutdown_errors.join("\n")
            ));
        }
        self.is_shutdown = true;
        Ok(())
    }
}

impl Drop for Providers {
    fn drop(&mut self) {
        if !self.is_shutdown {
            panic!("Telemetry providers was dropped without being shutdown!");
        }
    }
}
