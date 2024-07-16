#[derive(Clone, Copy)]
pub struct LogPlugin;

impl Default for LogPlugin {
    fn default() -> Self {
        Self
    }
}

impl app::plugins::Plugin for LogPlugin {
    fn build(&mut self, _app: &mut app::app::App) {
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
            util::tracing_subscriber::filter::EnvFilter::builder()
                .parse_lossy("trace,wgpu=warn,naga=warn,polling=error,winit=warn,calloop=warn"),
        );
        // let subscriber = subscriber.with(util::tracing_error::ErrorLayer::default());
        let subscriber = subscriber.with(
            util::tracing_subscriber::fmt::Layer::default()
                .without_time()
                .with_writer(std::io::stderr),
        );
        // let subscriber = subscriber.with(util::tracing_tracy::TracyLayer::default());

        use util::tracing_subscriber::layer::SubscriberExt;
        util::tracing::subscriber::set_global_default(subscriber).expect("setup tracing");
    }
}
