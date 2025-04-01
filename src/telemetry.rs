use opentelemetry::global;
use opentelemetry::trace::TraceContextExt as _;
use opentelemetry::trace::Tracer as _;
use opentelemetry::InstrumentationScope;
use opentelemetry::KeyValue;
use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use opentelemetry_otlp::SpanExporter;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_otlp::WithExportConfig as _;
use opentelemetry_otlp::{LogExporter, MetricExporter, Protocol};
use opentelemetry_sdk::trace::SdkTracerProvider;
use opentelemetry_sdk::{logs::SdkLoggerProvider, metrics::SdkMeterProvider, Resource};
use tracing::info;
use tracing::instrument::WithSubscriber;
use tracing::Subscriber;
use tracing_subscriber::{
    fmt::SubscriberBuilder, layer::SubscriberExt as _, util::SubscriberInitExt as _, EnvFilter,
    Layer,
};

const TELEMETRY_ENDPOINT_BASE: &str = "http://localhost:14268/api";

fn init_logs(resource: Resource) -> SdkLoggerProvider {
    let exporter = LogExporter::builder()
        .with_http()
        .with_endpoint(format!("{TELEMETRY_ENDPOINT_BASE}/v2/spans"))
        .build()
        .expect("Failed to create log exporter");

    SdkLoggerProvider::builder()
        .with_resource(resource)
        .with_batch_exporter(exporter)
        .build()
}

fn init_traces(resource: Resource) -> SdkTracerProvider {
    let exporter = SpanExporter::builder()
        .with_http()
        .with_endpoint(format!("{TELEMETRY_ENDPOINT_BASE}/v1/traces"))
        .with_protocol(Protocol::HttpBinary) //can be changed to `Protocol::HttpJson` to export in JSON format
        .build()
        .expect("Failed to create trace exporter");

    SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .with_resource(resource)
        .build()
}

fn init_metrics(resource: Resource) -> SdkMeterProvider {
    // FIXME: can't put metrics in jaeger?
    let exporter = MetricExporter::builder()
        .with_http()
        .with_protocol(Protocol::HttpBinary) //can be changed to `Protocol::HttpJson` to export in JSON format
        .build()
        .expect("Failed to create metric exporter");

    SdkMeterProvider::builder()
        .with_periodic_exporter(exporter)
        .with_resource(resource)
        .build()
}

pub fn init(service_name: &'static str) {
    let resource = Resource::builder().with_service_name(service_name).build();
    let exporter = init_logs(resource.clone());
    let otel_layer = OpenTelemetryTracingBridge::new(&exporter);

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

    // TODO: shutdown telemetry
}
