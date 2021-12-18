use core::fmt::{self, Write};
use lazy_static::lazy_static;
use spin::Mutex;
use volatile::Volatile;
use x86_64::instructions::port::Port;

lazy_static! {
    pub static ref WRITER: Mutex<Writer> = Mutex::new(Writer {
        row_position: 0,
        column_position: 0,
        color_code: ColorCode::new(Color::White, Color::Black),
        buffer: unsafe { &mut *(0xb8000 as *mut Buffer) },
    });
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Color {
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Magenta = 5,
    Brown = 6,
    LightGray = 7,
    DarkGray = 8,
    LightBlue = 9,
    LightGreen = 10,
    LightCyan = 11,
    LightRed = 12,
    Pink = 13,
    Yellow = 14,
    White = 15,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct ColorCode(u8);

impl ColorCode {
    pub const fn new(foreground: Color, background: Color) -> ColorCode {
        ColorCode((background as u8) << 4 | (foreground as u8))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
struct ScreenChar {
    ascii_character: u8,
    color_code: ColorCode,
}

const BUFFER_HEIGHT: usize = 25;
const BUFFER_WIDTH: usize = 80;

#[repr(transparent)]
struct Buffer {
    chars: [[Volatile<ScreenChar>; BUFFER_WIDTH]; BUFFER_HEIGHT],
}
pub struct Writer {
    row_position: usize,
    column_position: usize,
    color_code: ColorCode,
    buffer: &'static mut Buffer,
}

#[allow(dead_code)]
impl Writer {
    pub fn write_byte(&mut self, byte: u8, style: ColorCode) {
        match byte {
            b'\n' => self.new_line(),
            byte => {
                let row = self.row_position;
                let col = self.column_position;

                self.write_byte_at(byte, row, col, style);

                self.column_position += 1;
                if self.column_position >= BUFFER_WIDTH {
                    self.new_line();
                }
            }
        }
    }

    pub fn write_byte_at(&mut self, byte: u8, row: usize, col: usize, style: ColorCode) {
        self.buffer.chars[row][col].write(ScreenChar {
            ascii_character: byte,
            color_code: style,
        });
    }

    fn new_line(&mut self) {
        self.column_position = 0;
        self.row_position += 1;

        if self.row_position >= BUFFER_HEIGHT {
            self.scroll();
        }
    }

    fn scroll(&mut self) {
        for row in 0..(BUFFER_HEIGHT - 1) {
            for col in 0..BUFFER_WIDTH {
                let c = self.buffer.chars[row + 1][col].read();
                self.buffer.chars[row][col].write(c);
            }
        }

        self.clear_row(BUFFER_HEIGHT - 1);
        self.column_position = 0;
        self.row_position = BUFFER_HEIGHT - 1;
    }

    fn clear_row(&mut self, row: usize) {
        let clear_style = ScreenChar {
            ascii_character: b' ',
            color_code: self.color_code,
        };
        for col in 0..BUFFER_WIDTH {
            self.buffer.chars[row][col].write(clear_style);
        }
    }

    fn write_str(&mut self, s: &str, color: ColorCode) {
        for byte in s.bytes() {
            self.write_byte(byte, color);
        }

        let row = self.row_position;
        let col = self.column_position;
        self.move_cursor(row, col);
    }

    pub fn set_color(&mut self, color: ColorCode) {
        self.color_code = color
    }

    pub fn color(&self) -> ColorCode {
        self.color_code
    }

    pub fn return_color(&mut self) -> ColorWriter {
        ColorWriter {
            color_code: self.color_code,
            inner: self,
        }
    }

    fn move_cursor(&mut self, row: usize, col: usize) {
        assert!(
            row < BUFFER_HEIGHT,
            "attempted out-of-bounds (row) cursor move"
        );
        assert!(
            col < BUFFER_WIDTH,
            "attempted out-of-bounds (col) cursor move"
        );

        let pos: u16 = ((row * 80) + col) as u16;
        unsafe {
            Port::new(0x3d4).write(0x0Fu8);
            Port::new(0x3d5).write((pos & 0xff) as u8);

            Port::new(0x3d4).write(0x0eu8);
            Port::new(0x3d5).write(((pos >> 8) & 0xff) as u8);
        }
    }

    pub fn clear_screen(&mut self) {
        let clear_style = ScreenChar {
            ascii_character: b' ',
            color_code: self.color_code,
        };
        for row in 0..BUFFER_HEIGHT {
            for col in 0..BUFFER_WIDTH {
                self.buffer.chars[row][col].write(clear_style);
            }
        }
    }
}

impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let color = self.color_code;
        self.write_str(s, color);
        Ok(())
    }
}

pub struct ColorWriter<'a> {
    inner: &'a mut Writer,
    color_code: ColorCode,
}

impl<'a> ColorWriter<'a> {
    pub fn set_color(mut self, color: ColorCode) -> Self {
        self.color_code = color;
        self
    }
}

impl<'a> fmt::Write for ColorWriter<'a> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.inner.write_str(s, self.color_code);
        Ok(())
    }
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::vga_buffer::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => (print!("\n"));
    ($fmt:expr) => (print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => (print!(concat!($fmt, "\n"), $($arg)*));
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use x86_64::instructions::interrupts;
    interrupts::without_interrupts(|| {
        let _ = WRITER.lock().write_fmt(args).unwrap();
    })
}
