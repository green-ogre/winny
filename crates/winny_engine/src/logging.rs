use std::time::{SystemTime, UNIX_EPOCH};

#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => {
        $crate::logging::print_log($crate::logging::LogLevel::Warn, format!($($arg)*));
    };
}

#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {
        $crate::logging::print_log($crate::logging::LogLevel::Info, format!($($arg)*));
    };
}

#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => {
        $crate::logging::print_log($crate::logging::LogLevel::Error, format!($($arg)*));
    };
}

pub enum LogLevel {
    Info,
    Warn,
    Error,
}

pub fn print_log(level: LogLevel, arg: String) {
    {
        #[cfg(debug_assertions)]
        {
            let time_stamp = SystemTime::now();
            let since_the_epoch = time_stamp.duration_since(UNIX_EPOCH).unwrap();
            let color = match level {
                LogLevel::Info => "\x1b[0;32m",
                LogLevel::Warn => "\x1b[0;33m",
                LogLevel::Error => "\x1b[0;31m",
            };

            println!(
                "[{:?}:{:?}:{:?}\t{}INFO\x1b[0m]\t{}",
                since_the_epoch.as_secs() / 3600,
                since_the_epoch.as_secs() / 60,
                since_the_epoch.as_secs(),
                color,
                arg
            );
        }
    }
}
