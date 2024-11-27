
#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

use core::panic::PanicInfo;

#[panic_handler]
fn panic(info: &PanicInfo)->!
{
    // When the OS will provide a "kill this process" function,
    // it will get called here.



    loop {}
}

#[no_mangle]
pub extern "C" fn _start()
{
    loop {}
}