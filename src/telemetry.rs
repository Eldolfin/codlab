use opentelemetry::global;
use opentelemetry::trace::TraceContextExt as _;
use opentelemetry::trace::Tracer as _;
use opentelemetry::InstrumentationScope;
use opentelemetry::KeyValue;
use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use opentelemetry_otlp::SpanExporter;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_otlp::{LogExporter, MetricExporter, Protocol};
use opentelemetry_sdk::trace::SdkTracerProvider;
use opentelemetry_sdk::{logs::SdkLoggerProvider, metrics::SdkMeterProvider, Resource};
use tracing::info;
use tracing_subscriber::{
    fmt::SubscriberBuilder, layer::SubscriberExt as _, util::SubscriberInitExt as _, EnvFilter,
    Layer,
};

const TRACES_ENDPOINT: &str = "http://localhost:4317";
const METRICS_ENDPOINT: &str = "http://localhost:4317/v1/metrics";

pub struct Providers {
    logs: SdkLoggerProvider,
    traces: SdkTracerProvider,
    metrics: SdkMeterProvider,
}

fn init_logs(resource: Resource) -> SdkLoggerProvider {
    let exporter = LogExporter::builder()
        .with_tonic()
        // .with_endpoint(TRACES_ENDPOINT)
        .build()
        .expect("Failed to create log exporter");

    SdkLoggerProvider::builder()
        .with_resource(resource)
        .with_batch_exporter(exporter)
        .build()
}

fn init_traces(resource: Resource) -> SdkTracerProvider {
    // FIXME: collect traces?
    let exporter = SpanExporter::builder()
        .with_tonic()
        // .with_endpoint(TRACES_ENDPOINT)
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
        // .with_endpoint(METRICS_ENDPOINT)
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

    let common_scope_attributes = vec![KeyValue::new("scope-key", "scope-value")];
    let scope = InstrumentationScope::builder("basic")
        .with_version("1.0")
        .with_attributes(common_scope_attributes)
        .build();

    let tracer = global::tracer_with_scope(scope.clone());
    let meter = global::meter_with_scope(scope);

    let counter = meter
        .u64_counter("test_counter")
        .with_description("a simple counter for demo purposes.")
        .with_unit("my_unit")
        .build();

    counter.add(1, &[KeyValue::new("test_key", "test_value")]);
    tracer.in_span("Main operation", |cx| {
        let span = cx.span();
        span.add_event(
            "Nice operation!".to_string(),
            vec![KeyValue::new("some.key", 100)],
        );
        span.set_attribute(KeyValue::new("another.key", "yes"));

        info!(target: "my-target", "hello from {}. My price is {}. I am also inside a Span!", "banana", 2.99);

        tracer.in_span("Sub operation...", |cx| {
            let span = cx.span();
            span.set_attribute(KeyValue::new("another.key", "yes"));
            span.add_event("Sub span event", vec![]);
        });
    });

    Providers {
        traces: tracer_provider,
        metrics: meter_provider,
        logs: logger_provider,
    }
    .shutdown()
    .unwrap();
    todo!()
}

impl Providers {
    pub fn shutdown(self) -> anyhow::Result<()> {
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
        Ok(())
    }
}
