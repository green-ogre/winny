#[cfg(target_arch = "wasm32")]
use util::tracing::{
    field::{Field, Visit},
    Subscriber,
};

#[derive(Debug, Clone, Copy)]
pub struct LogPlugin {
    pub level: &'static str,
}

impl Default for LogPlugin {
    fn default() -> Self {
        Self { level: "info" }
    }
}

impl app::plugins::Plugin for LogPlugin {
    fn build(&mut self, _app: &mut app::prelude::App) {
        // let old_handler = std::panic::take_hook();
        // std::panic::set_hook(Box::new(move |infos| {
        //     util::tracing::error!("{}", util::tracing_error::SpanTrace::capture());
        //     old_handler(infos);
        // }));
        // std::panic::set_hook(Box::new(|panic_info| {
        //     let backtrace = std::backtrace::Backtrace::force_capture();
        //     util::tracing::error!("My backtrace: {:#?}", backtrace);
        // }));

        util::tracing_log::LogTracer::init().unwrap();

        let subscriber = util::tracing_subscriber::Registry::default();
        let subscriber = subscriber.with(
            util::tracing_subscriber::filter::EnvFilter::builder().parse_lossy(&format!(
                "{},wgpu=warn,naga=warn,polling=error,winit=warn,calloop=warn",
                self.level
            )),
        );
        // let subscriber = subscriber.with(util::tracing_error::ErrorLayer::default());
        let subscriber = subscriber.with(
            util::tracing_subscriber::fmt::Layer::default()
                .without_time()
                .with_writer(std::io::stderr),
        );
        // let subscriber = subscriber.with(util::tracing_tracy::TracyLayer::default());

        #[cfg(target_arch = "wasm32")]
        let subscriber = subscriber.with(ConsoleLog {});

        use util::tracing_subscriber::layer::SubscriberExt;
        util::tracing::subscriber::set_global_default(subscriber).expect("setup tracing");
    }
}

#[cfg(target_arch = "wasm32")]
#[derive(Default)]
struct ConsoleLog;

#[cfg(target_arch = "wasm32")]
#[derive(Default)]
pub struct ConsoleVisitor {
    result: String,
}

#[cfg(target_arch = "wasm32")]
impl Visit for ConsoleVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        self.result = format!("{}: {:?},", field.name(), value);
    }
}

#[cfg(target_arch = "wasm32")]
impl<S: Subscriber> util::tracing_subscriber::Layer<S> for ConsoleLog {
    fn on_event(
        &self,
        event: &util::tracing::Event<'_>,
        _ctx: util::tracing_subscriber::layer::Context<'_, S>,
    ) {
        let mut visitor = ConsoleVisitor::default();
        event.record(&mut visitor);
        web_sys::console::log_2(
            &format!("{:?}", event.metadata().level()).into(),
            &visitor.result.into(),
        );
    }
}
