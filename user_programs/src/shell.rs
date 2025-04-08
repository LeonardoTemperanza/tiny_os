
#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(dead_code)]

use core::panic::PanicInfo;
mod tinyos_userlib;
use crate::tinyos_userlib::*;

#[panic_handler]
fn panic(info: &PanicInfo)->!
{
    println("Panicking!");
    loop {}
}

#[unsafe(no_mangle)]
pub extern "C" fn _start()
{
    println("Welcome to TinyOS! I'm a user-program \"shell\".");
    println("Type 'help' for a list of available commands.");

    print_num(3);
    println("");

    loop {}
}
