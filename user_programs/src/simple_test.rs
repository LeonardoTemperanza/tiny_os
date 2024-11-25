
#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

use core::panic::PanicInfo;

#[panic_handler]
fn panic(info: &PanicInfo)->!
{
    loop {}
}

#[no_mangle]
pub extern "C" fn _start()
{
    
}