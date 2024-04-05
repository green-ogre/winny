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
macro_rules! trace {
    ($($arg:tt)*) => {
        $crate::print_log($crate::LogLevel::Trace, file!(), line!(), format!($($arg)*));
    };
}

#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => {
        $crate::print_log($crate::LogLevel::Error, file!(), line!(), format!($($arg)*));
    };
}

pub enum LogLevel {
    Trace,
    Info,
    Warn,
    Error,
}

pub fn print_log(level: LogLevel, file: &str, line: u32, arg: String) {
    {
        // #[cfg(debug_assertions)]
        {
            let msg_level = match level {
                LogLevel::Trace => "\x1b[38;2;128;128;128mTRACE\x1b[0m",
                LogLevel::Info => "\x1b[0;32mINFO\x1b[0m",
                LogLevel::Warn => "\x1b[0;33mWARN\x1b[0m",
                LogLevel::Error => "\x1b[0;31mERROR\x1b[0m",
            };

            let trace_gray = match level {
                LogLevel::Trace => "\x1b[38;2;128;128;128m",
                _ => "",
            };

            println!(
                "[{}:{:?} {}]{} {}\x1b[0m",
                file, line, msg_level, trace_gray, arg
            );
        }
    }
}
