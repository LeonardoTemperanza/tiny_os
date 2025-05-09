
#![no_std]
#![feature(abi_x86_interrupt)]
#![feature(naked_functions)]
#![feature(str_from_raw_parts)]
#![allow(unsafe_op_in_unsafe_fn)]

#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]

extern crate alloc;
use core::panic::PanicInfo;

pub mod allocator;
pub mod gdt;
pub mod interrupts;
pub mod memory;
pub mod serial;
pub mod vga_buffer;
pub mod process;
pub mod base;

pub fn init()
{
    gdt::init();
    interrupts::init_idt();
    unsafe { interrupts::PICS.lock().initialize() };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum QemuExitCode {
    Success = 0x10,
    Failed = 0x11,
}

pub fn exit_qemu(exit_code: QemuExitCode)
{
    use x86_64::instructions::port::Port;

    unsafe {
        let mut port = Port::new(0xf4);
        port.write(exit_code as u32);
    }
}

pub fn hlt_loop() -> !
{
    loop { x86_64::instructions::hlt(); }
}
