
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
    println("About to compute the 10th fibonacci recursively...");
    let res = rec_fib(9);
    print("Result: ");
    print_num(res);
    println("");

    exit(0);
}

pub fn rec_fib(n: u64) -> u64
{
    if n == 0 { return 0; }
    if n == 1 { return 1; }
    return rec_fib(n-1) + rec_fib(n-2);
}
