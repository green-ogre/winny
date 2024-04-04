#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => {
        $crate::print_log($crate::LogLevel::Warn, file!(), line!(), format!($($arg)*));
    };
}

#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {
        $crate::print_log($crate::LogLevel::Info, file!(), line!(), format!($($arg)*));
    };
}

#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => {
        $crate::print_log($crate::LogLevel::Error, file!(), line!(), format!($($arg)*));
    };
}

pub enum LogLevel {
    Info,
    Warn,
    Error,
}

pub fn print_log(level: LogLevel, file: &str, line: u32, arg: String) {
    {
        #[cfg(debug_assertions)]
        {
            let color = match level {
                LogLevel::Info => "\x1b[0;32m",
                LogLevel::Warn => "\x1b[0;33m",
                LogLevel::Error => "\x1b[0;31m",
            };

            println!("[{}:{:?} {}INFO\x1b[0m] {}", file, line, color, arg);
        }
    }
}
