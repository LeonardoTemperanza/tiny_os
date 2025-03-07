
pub const USER_PROGRAM: &[u8; 8816] = include_bytes!("simple_test");
pub const USER_PROGRAM_NAME: &str = "simple_test";

use crate::println;
use crate::base::*;
use crate::memory;
use crate::interrupts;
use core::mem::{size_of};
use core::arch::asm;
use core::{pin::Pin};
use alloc::{boxed::Box, vec::Vec};
use crate::allocator;
use lazy_static::lazy_static;

use buddy_system_allocator as heap;

#[derive(Default, Clone, Copy, Debug)]
pub struct ElfHeader
{
    //magic_num: [u8; 4],      // 0x7F, ELF
    is_64_bits: u8,            // 1 = 32 bits, 2 = 64 bits
    endianness: u8,            // 1 = little_endian, 2 = big_endian
    elf_header_version: u8,
    os_abi: u8,                // Usually 0 for System V
    _unused: u64,
    bin_type: u16,             // 1 = relocatable, 2 = executable, 3 = shared, 4 = core
    isa: u16,
    elf_version: u32,          // 1
    entry_vaddr: u64,
    pht_offset: u64,
    sht_offset: u64,
    flags: u32,                // Architecture dependent
    elf_header_size: u16,
    pht_entry_size:  u16,
    pht_num_entries: u16,
    sht_entry_size:  u16,
    sht_num_entries: u16,
    section_names_index: u16,  // Entry in the sht which contains the names
}

#[derive(Default, Clone, Copy, Debug)]
pub struct ProgramHeader
{
    segment_type: u32,
    flags:  u32,
    offset: u64,
    vaddr:  u64,
    paddr:  u64,
    size_in_file: u64,
    size_in_memory: u64,
}

use x86_64::
{
    structures::paging::*,
    structures::paging::Page,
    structures::paging::mapper::MapToError,
    VirtAddr, PhysAddr
};

pub const USER_STACK_START: u64 = 0x800000;

pub fn create_task(blob: &[u8], mapper: &mut impl Mapper<Size4KiB>, phys_mem_offset: VirtAddr,
                   frame_allocator: &mut impl FrameAllocator<Size4KiB>, kernel_pagetable_phys_addr: PhysAddr) -> Option<Task>
{
    println!("Started to parse elf binary!");

    let elf_header = parse_elf_binary(blob);
    if elf_header.is_none() { return None; }
    let elf_header = elf_header.unwrap();

    if elf_header.is_64_bits != 2
    {
        println!("32 bits format is not supported.");
        return None;
    }

    if elf_header.endianness != 1
    {
        println!("Big endian format is not supported.");
        return None;
    }

    if elf_header.bin_type != 2
    {
        println!("Supported binary type used. Only executables are supported.");
        return None;
    }

    let pt = unsafe { memory::clone_page_table(kernel_pagetable_phys_addr, frame_allocator, phys_mem_offset).unwrap() };
    unsafe { memory::activate_page_table(pt) };

    println!("Activated page table!");

    //let mut free_interval_begin = allocator::USER_HEAP_START;
    for i in 0..elf_header.pht_num_entries
    {
        //println!("iter");

        let ph_offset = elf_header.pht_offset + (i as u64) * (elf_header.pht_entry_size as u64);
        let program_header = parse_elf_program_header(&blob[ph_offset as usize..]);
        print_elf_program_header(program_header);
        if program_header.segment_type == 1  // PT_LOAD segment
        {
            use x86_64::structures::paging::Page;
            let vaddr = program_header.vaddr as u64;
            let size_mem  = program_header.size_in_memory as u64;
            let size_file = program_header.size_in_file as u64;
            let start_page: Page = Page::containing_address(VirtAddr::new(vaddr));
            let end_page:   Page = Page::containing_address(VirtAddr::new(vaddr+size_mem));
            let page_range = Page::range_inclusive(start_page, end_page);
            assert!(size_file <= size_mem);

            // Map pages
            for page in page_range
            {
                let flags = elf_flags_to_page_table_flags(program_header.flags);
                let frame = frame_allocator.allocate_frame().unwrap();
                unsafe {
                    let res = mapper.map_to(page, frame, flags, frame_allocator).unwrap().flush();
                }

                // Copy the data over. Using the offset mapping because the mapping
                // that was just created might not have WRITABLE flag active!
                let dst = (phys_mem_offset + frame.start_address().as_u64()).as_mut_ptr() as *mut u8;
                let src = (&blob[program_header.offset as usize]) as *const u8;
                let phys_size = frame.size();
                unsafe { core::ptr::copy(src, dst, phys_size as usize) };
            }

            // Progressively add intervals into the process allocator
            //if start_page.start_address().as_u64() as usize - free_interval_begin > 0 {
                //unsafe { process_allocator.lock().add_to_heap(free_interval_begin, (start_page.start_address().as_u64()) as usize); }
            //}

            //free_interval_begin = (end_page.start_address().as_u64() + end_page.size() + 1) as usize;
        }
        else if program_header.segment_type == 2  // PT_DYNAMIC segment
        {
            println!("There is a segment that requires dynamic linking, which is not yet supported.");
            return None;
        }
        else
        {
            // There are others, but mostly contain architecture/environment specific info
        }
    }

    // Reserve memory for stack space
    let stack_phys_frame = frame_allocator.allocate_frame().unwrap();
    let stack_virt_page = Page::containing_address(VirtAddr::new(USER_STACK_START));
    let stack_flags = PageTableFlags::PRESENT | PageTableFlags::USER_ACCESSIBLE | PageTableFlags::WRITABLE;
    unsafe {
        mapper.map_to(stack_virt_page, stack_phys_frame, stack_flags, frame_allocator).unwrap().flush();
    }

    return Some(Task {
        started: false,
        ctx: Default::default(),
        start_instr: VirtAddr::new(elf_header.entry_vaddr),
        stack_ptr: VirtAddr::new(USER_STACK_START),
        //page_table: ,
    });

    //unsafe { jump_to_entry_point(elf_header.entry_vaddr) };
}

pub fn jump_to_usercode(code: VirtAddr, stack_end: VirtAddr)
{
    //let (cs_idx, ds_idx) = gdt::set_usermode
}

pub fn elf_flags_to_page_table_flags(flags: u32) -> PageTableFlags
{
    let mut res = PageTableFlags::PRESENT | PageTableFlags::USER_ACCESSIBLE;
    if flags & 1 == 0 { res &= PageTableFlags::NO_EXECUTE }
    if flags & 2 != 0 { res &= PageTableFlags::WRITABLE   }
    if flags & 4 != 0 { }
    return res;
}

pub fn parse_elf_binary(blob: &[u8]) -> Option<ElfHeader>
{
    let mut r = Reader::new(blob);

    let mut header = ElfHeader::default();

    let mut magic_bytes: &str = "";
    r.read_len_string(&mut magic_bytes, 4);

    if magic_bytes != "\u{7F}ELF"
    {
        println!("The supplied binary is not ELF.");
        return None;
    }

    r.read(&mut header.is_64_bits);
    r.read(&mut header.endianness);
    r.read(&mut header.elf_header_version);
    r.read(&mut header.os_abi);
    r.read(&mut header._unused);
    r.read(&mut header.bin_type);
    r.read(&mut header.isa);
    r.read(&mut header.elf_version);
    r.read(&mut header.entry_vaddr);
    r.read(&mut header.pht_offset);
    r.read(&mut header.sht_offset);
    r.read(&mut header.flags);
    r.read(&mut header.elf_header_size);
    r.read(&mut header.pht_entry_size);
    r.read(&mut header.pht_num_entries);
    r.read(&mut header.sht_entry_size);
    r.read(&mut header.sht_num_entries);
    r.read(&mut header.section_names_index);
    return Some(header);
}

pub fn parse_elf_program_header(blob: &[u8]) -> ProgramHeader
{
    let mut r = Reader::new(blob);
    let mut header = ProgramHeader::default();

    r.read(&mut header.segment_type);
    r.read(&mut header.flags);
    r.read(&mut header.offset);
    r.read(&mut header.vaddr);
    r.read(&mut header.paddr);
    r.read(&mut header.size_in_file);
    r.read(&mut header.size_in_memory);
    return header;
}

// Scheduler

pub struct Task
{
    started: bool,

    ctx: Context,

    start_instr: VirtAddr,
    stack_ptr:   VirtAddr,
    //page_table: PageTable,

    // Need some more info for destruction of task
}

impl Drop for Task
{
    fn drop(self: &mut Self) {}
}

pub struct Scheduler
{
    //tasks: Vec<Task>,
    //cur_task: usize,
}

lazy_static! {
    pub static ref SCHEDULER: Scheduler = Scheduler::new();
}

impl Scheduler
{
    pub fn new() -> Self
    {
        return Scheduler {};
    }

    pub unsafe fn schedule_data(&self, prog_data: Vec<u8>, entry_offset: usize)
    {

    }
}

// Context switching

#[derive(Default, Clone, Copy, Debug)]
pub struct Context
{
    pub rbp: u64,
    pub rax: u64,
    pub rbx: u64,
    pub rcx: u64,
    pub rdx: u64,
    pub rsi: u64,
    pub rdi: u64,
    pub r8: u64,
    pub r9: u64,
    pub r10: u64,
    pub r11: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,
    pub rip: u64,
    pub cs: u64,
    pub rflags: u64,
    pub rsp: u64,
    pub ss: u64,
}

#[inline(always)]
pub unsafe fn get_context() -> *const Context
{
    let ctx_ptr: *const Context;
    unsafe {
        asm!("push r15; push r14; push r13; push r12; push r11; push r10; push r9;\
        push r8; push rdi; push rsi; push rdx; push rcx; push rbx; push rax; push rbp;\
        mov {}, rsp; sub rsp, 0x400;", out(reg) ctx_ptr);
    }
    return ctx_ptr;
}

#[inline(always)]
pub unsafe fn restore_context_and_return_from_interrupt(ctx: &Context)
{
    unsafe {
        asm!("mov rsp, {};\
        pop rbp; pop rax; pop rbx; pop rcx; pop rdx; pop rsi; pop rdi; pop r8; pop r9;\
        pop r10; pop r11; pop r12; pop r13; pop r14; pop r15; iretq;", in(reg) ctx);
    }
}

pub unsafe extern "sysv64" fn context_switch(ctx: *const Context)
{
    //SCHEDULER.save_current_context(ctx);
    //interrupts::notify_end_of_timer_interrupt();
    //SCHEDULER.run_next();
}

// Debugging utils

pub fn print_elf_header(header: ElfHeader)
{
    println!("is_64_bits:{}, endianness:{}, version:{}, os_abi:{}, bin_type:{}, isa:{}, elf_version:{}, entry_vaddr:{}, pht_offset:{}, sht_offset:{}, flags:{}, header_size:{}, pht_entry_size:{}, pht_num_entries:{}, sht_entry_size:{}, sht_num_entries:{}, section_names_index:{}",
            header.is_64_bits, header.endianness,
            header.elf_header_version, header.os_abi,
            header.bin_type, header.isa,
            header.elf_version, header.entry_vaddr,
            header.pht_offset, header.sht_offset,
            header.flags, header.elf_header_size,
            header.pht_entry_size, header.pht_num_entries,
            header.sht_entry_size, header.sht_num_entries,
            header.section_names_index);
}

pub fn print_elf_program_header(header: ProgramHeader)
{
    println!("segment_type: {}, flags: {}, offset: {}, vaddr: {}, paddr: {}, size_in_file: {}, size_in_memory: {}",
        header.segment_type, header.flags, header.offset,
        header.vaddr, header.paddr,
        header.size_in_file, header.size_in_memory,
    );
}
