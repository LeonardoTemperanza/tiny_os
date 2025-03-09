
#![no_std]
#![no_main]
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]

extern crate alloc;

use alloc::{boxed::Box, rc::Rc, vec, vec::Vec};
use tinyos::println;
use bootloader::{entry_point, BootInfo};
use core::panic::PanicInfo;
use tinyos::process;

entry_point!(kernel_main);

fn kernel_main(boot_info: &'static BootInfo) -> !
{
    use tinyos::allocator;
    use tinyos::memory::{self, BootInfoFrameAllocator};
    use x86_64::VirtAddr;

    tinyos::init();

    let phys_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let mut kernel_page_table = unsafe { memory::init_kernel_page_table(phys_offset) };
    let kernel_page_table_phys_addr = unsafe { memory::active_level_4_table_addr() };
    let mut frame_allocator = unsafe { BootInfoFrameAllocator::init(&boot_info.memory_map) };

    allocator::init_heap(&mut kernel_page_table, &mut frame_allocator).expect("Heap initialization failed.");

    let task = process::create_task(process::USER_PROGRAM, &mut kernel_page_table, phys_offset, &mut frame_allocator, kernel_page_table_phys_addr);
    process::SCHEDULER.lock().schedule_task(task.unwrap());
    //unsafe { process::SCHEDULER.lock().run_next_task() };

    println!("End of main");
    tinyos::hlt_loop();
    //panic!("Reached end of main!");
}

#[panic_handler]
fn panic(info: &PanicInfo) -> !
{
    println!("{}", info);
    tinyos::hlt_loop();
}
