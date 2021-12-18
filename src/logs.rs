use log::{self, Level, LevelFilter, Log, Metadata, Record, SetLoggerError};
use x86_64::instructions::interrupts;

pub const LOG_LEVEL: log::Level = log::Level::Debug;

static LOGGER: Logger = Logger;

pub fn init() -> Result<(), SetLoggerError> {
    log::set_logger(&LOGGER)?;
    log::set_max_level(LOGGER.filter());
    Ok(())
}

struct Logger;

impl Logger {
    fn filter(&self) -> LevelFilter {
        LOG_LEVEL.to_level_filter()
    }
}

impl Log for Logger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= LOG_LEVEL
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            // #[cfg(feature = "logging-serial")]
            // {
            // use core::fmt::Write;
            // interrupts::without_interrupts(|| {
            //     writeln!(
            //         crate::serial::COM1.write(),
            //         "[{}]: {}",
            //         record.level(),
            //         record.args()
            //     )
            //     .unwrap()
            // });
            // }
            // #[cfg(feature = "logging-console")]
            // {
            use crate::vga_buffer::{Color, ColorCode, WRITER};
            use core::fmt::Write;

            let color = ColorCode::new(
                match record.level() {
                    Level::Error => Color::Red,
                    Level::Warn => Color::Magenta,
                    Level::Info => Color::Green,
                    Level::Debug => Color::Cyan,
                    Level::Trace => Color::White,
                },
                Color::Black,
            );

            interrupts::without_interrupts(|| {
                let mut wtr = WRITER.lock();
                write!(wtr.return_color().set_color(color), "{:>5}", record.level()).unwrap();

                writeln!(wtr, ": {}", record.args()).unwrap();
            });
            // }
        }
    }

    fn flush(&self) {}
}
