
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
use lazy_static::lazy_static;
use tinyos::allocator;
use tinyos::memory::{self, BootInfoFrameAllocator};
use x86_64::{VirtAddr, PhysAddr};

entry_point!(kernel_main);

fn kernel_main(boot_info: &'static BootInfo) -> !
{
    tinyos::init();

    // Get memory info from boot loader
    let mut kernel_page_table;
    {
        let mut kernel_mem_info = memory::KERNEL_MEM_INFO.lock();
        (*kernel_mem_info).phys_offset = VirtAddr::new(boot_info.physical_memory_offset);
        //phys_offset = VirtAddr::new(boot_info.physical_memory_offset);
        kernel_page_table = unsafe { memory::init_kernel_page_table(kernel_mem_info.phys_offset) };
        (*kernel_mem_info).kernel_page_table_phys_addr = unsafe { memory::active_level_4_table_addr() };
        //kernel_page_table_phys_addr = unsafe { memory::active_level_4_table_addr() };
    }

    // Init frame allocator
    {
        let mut frame_allocator = memory::FRAME_ALLOCATOR.lock();
        *frame_allocator = unsafe { BootInfoFrameAllocator::init(&boot_info.memory_map) };
    }

    allocator::init_heap(&mut kernel_page_table).expect("Heap initialization failed.");

    {
        let kern_mem_info = memory::KERNEL_MEM_INFO.lock();
        let task = process::create_task(process::USER_PROGRAM_SHELL, kern_mem_info.phys_offset, kern_mem_info.kernel_page_table_phys_addr, 0);
        println!("Created task.");

        process::SCHEDULER.schedule_task(task.unwrap());
        println!("Scheduled task.");
    }

    // We will be interrupted soon
    println!("End of main.");
    x86_64::instructions::interrupts::enable();
    tinyos::hlt_loop();
}

#[panic_handler]
fn panic(info: &PanicInfo) -> !
{
    println!("{}", info);
    tinyos::hlt_loop();
}
