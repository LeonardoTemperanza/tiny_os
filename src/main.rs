
#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

use core::panic::PanicInfo;
use core::fmt;
use core::fmt::Write;
use volatile::Volatile;
use lazy_static::lazy_static;
use spin::Mutex;

mod interrupts;
mod memory;

use bootloader::{BootInfo, entry_point};

entry_point!(kernel_main);

fn kernel_main(boot_info: &'static BootInfo) -> !
{
    kernel_init(boot_info);

    use x86_64::VirtAddr;
    use x86_64::structures::paging::{OffsetPageTable, Mapper, Translate};

    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);

    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator = memory::EmptyFrameAllocator;

    // Map an unused page
    use x86_64::structures::paging::Page;
    //let page = Page::containing_address(VirtAddr::new(0));
    let page = Page::containing_address(VirtAddr::new(0xdeadbeef000));
    memory::create_example_mapping(page, &mut mapper, &mut frame_allocator);

    let page_ptr: *mut u64 = page.start_address().as_mut_ptr();
    unsafe { page_ptr.offset(400).write_volatile(0x_f021_f077_f065_f04e)};

    println!("Reached end of kernel main.");
    halt_loop();
}

fn kernel_init(boot_info: &'static BootInfo)
{
    interrupts::init_idt();
    println!("Interrupt Descriptor Table initialized.");

    interrupts::init_gdt();
    println!("Global Descriptor Table initalized.");

    // PICS Initialized
    unsafe { interrupts::PICS.lock().initialize() };
    println!("PICS initialized");

    x86_64::instructions::interrupts::enable();
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> !
{
    println!("{}", _info);    
    halt_loop();
}

fn halt_loop() -> !
{
    loop { x86_64::instructions::hlt(); }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Color
{
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

fn to_color_code(foreground: Color, background: Color)->u8
{
    return (background as u8) << 4 | (foreground as u8);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
struct ScreenChar
{
    ascii_character: u8,
    color_code: u8,
}

const BUFFER_HEIGHT: usize = 25;
const BUFFER_WIDTH:  usize = 80;

#[repr(transparent)]
struct Buffer
{
    chars: [[Volatile<ScreenChar>; BUFFER_WIDTH]; BUFFER_HEIGHT],
}

pub struct Writer
{
    column_position: usize,
    color_code: u8,
    buffer: &'static mut Buffer,
}

impl Writer
{
    pub fn write_byte(&mut self, byte: u8)
    {
        match byte
        {
            b'\n' => self.new_line(),
            byte  =>
            {
                if self.column_position >= BUFFER_WIDTH { self.new_line(); }

                let row = BUFFER_HEIGHT - 1;
                let col = self.column_position;

                let color_code = self.color_code;
                self.buffer.chars[row][col] = Volatile::new(ScreenChar
                {
                    ascii_character: byte,
                    color_code,
                });

                self.column_position += 1;
            }
        }
    }

    pub fn write_string(&mut self, s: &str)
    {
        for byte in s.bytes()
        {
            match byte
            {
                // printable ASCII byte or newline
                0x20..=0x7e | b'\n' => self.write_byte(byte),
                // not part of printable ASCII range
                _ => self.write_byte(0xfe),
            }
        }
    }

    fn new_line(&mut self)
    {
        for row in 1..BUFFER_HEIGHT
        {
            for col in 0..BUFFER_WIDTH
            {
                let character = self.buffer.chars[row][col].read();
                self.buffer.chars[row - 1][col].write(character);
            }
        }

        self.clear_row(BUFFER_HEIGHT - 1);
        self.column_position = 0;
    }

    fn clear_row(&mut self, row: usize)
    {
        let blank = ScreenChar
        {
            ascii_character: b' ',
            color_code: self.color_code,
        };

        for col in 0..BUFFER_WIDTH
        {
            self.buffer.chars[row][col].write(blank);
        }
    }
}

impl fmt::Write for Writer
{
    fn write_str(&mut self, s: &str)->fmt::Result
    {
        self.write_string(s);
        return Ok(());
    }
}

lazy_static!
{
    pub static ref WRITER: Mutex<Writer> = Mutex::new(Writer
    {
        column_position: 0,
        color_code: to_color_code(Color::White, Color::Black),
        buffer: unsafe { &mut *(0xb8000 as *mut Buffer) }
    });
}

#[macro_export]
macro_rules! print
{
    ($($arg:tt)*) => ($crate::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println
{
    () => (print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments)
{
    use core::fmt::Write;
    WRITER.lock().write_fmt(args).unwrap();
}
