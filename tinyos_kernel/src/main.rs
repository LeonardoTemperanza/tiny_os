
#![no_std]
#![no_main]
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]

extern crate alloc;

use tinyos::println;
use bootloader::{entry_point, BootInfo};
use core::panic::PanicInfo;

entry_point!(kernel_main);

fn kernel_main(boot_info: &'static BootInfo) -> !
{
    use tinyos::allocator;
    use tinyos::memory::{self, BootInfoFrameAllocator};
    use x86_64::VirtAddr;

    tinyos::init();

    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator = unsafe { BootInfoFrameAllocator::init(&boot_info.memory_map) };

    allocator::init_heap(&mut mapper, &mut frame_allocator).expect("heap initialization failed");

    println!("Initialization done!");

    println!("End of main 1!");
    println!("End of main 2!");
    println!("End of main 3!");
    println!("End of main 4!");
    println!("End of main 5!");
    println!("End of main 6!");
    println!("End of main 7!");
    println!("End of main 8!");
    println!("End of main 9!");
    println!("End of main 10!");
    println!("End of main 11!");
    println!("End of main 12!");
    println!("End of main 13!");
    println!("End of main 14!");
    println!("End of main 15!");
    println!("End of main 16!");
    println!("End of main 17!");
    println!("End of main 18!");
    println!("End of main 19!");
    println!("End of main 20!");
    println!("End of main 21!");
    println!("End of main 22!");
    println!("End of main 23!");
    println!("End of main 24!");
    println!("End of main 25!");
    println!("end.");
    tinyos::hlt_loop();
}

#[panic_handler]
fn panic(info: &PanicInfo) -> !
{
    println!("{}", info);
    tinyos::hlt_loop();
}
