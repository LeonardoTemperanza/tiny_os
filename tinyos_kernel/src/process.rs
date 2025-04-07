
pub const USER_PROGRAM: &[u8] = include_bytes!("simple_test");
pub const USER_PROGRAM_NAME: &str = "simple_test";

use crate::println;
use crate::print;
use crate::base::*;
use crate::memory;
use crate::interrupts;
use core::mem::{size_of};
use core::arch::asm;
use core::{pin::Pin};
use alloc::{boxed::Box, vec::Vec};
use crate::allocator;
use lazy_static::lazy_static;
use spin::Mutex;

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

pub fn create_task(blob: &[u8], mapper: &mut impl Mapper<Size4KiB>, phys_offset: VirtAddr,
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
        println!("Unsupported binary type used. Only executables are supported.");
        return None;
    }

    let pt = unsafe { memory::clone_page_table(kernel_pagetable_phys_addr, frame_allocator, phys_offset).unwrap() };
    let pt_virt = phys_offset + pt.as_u64();
    let pt_ptr: *mut PageTable = pt_virt.as_mut_ptr();
    let mut process_mapper = unsafe { OffsetPageTable::new(&mut *pt_ptr, phys_offset) };

    for i in 0..elf_header.pht_num_entries
    {
        println!("Iteration");
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
            for (i, page) in page_range.enumerate() {
                let page_vaddr = page.start_address().as_u64();

                let offset_in_segment = page_vaddr.saturating_sub(vaddr);

                let flags = elf_flags_to_page_table_flags(program_header.flags);
                print_page_flags(flags);
                let frame = frame_allocator.allocate_frame().unwrap();
                if frame.size() != 4096 { panic!("Assuming physical frame size is 4KB"); }

                unsafe {
                    println!("About to map page {} to {}", page.start_address().as_u64(), frame.start_address().as_u64());
                    process_mapper.map_to(page, frame, flags, frame_allocator).unwrap().flush();
                }

                let dst = (phys_offset + frame.start_address().as_u64()).as_mut_ptr() as *mut u8;

                // Zero the entire page
                unsafe { core::ptr::write_bytes(dst, 0, frame.size() as usize) };

                if offset_in_segment < size_file {
                    let bytes_to_copy = core::cmp::min(
                        size_file - offset_in_segment,
                        frame.size() as u64
                    ) as usize;

                    let file_offset = program_header.offset + offset_in_segment;

                    if (file_offset as usize) < blob.len() {
                        let safe_bytes = core::cmp::min(
                            bytes_to_copy,
                            blob.len() - file_offset as usize
                        );

                        let src = &blob[file_offset as usize] as *const u8;

                        unsafe {
                            core::ptr::copy_nonoverlapping(src, dst, safe_bytes);
                        }
                    }
                }
            }
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
        process_mapper.map_to(stack_virt_page, stack_phys_frame, stack_flags, frame_allocator).unwrap().flush();
        //mapper.map_to(stack_virt_page, stack_phys_frame, stack_flags, frame_allocator).unwrap().flush();
    }

    return Some(Task {
        started: false,
        ctx: Default::default(),
        start_instr: VirtAddr::new(elf_header.entry_vaddr),
        stack_end: VirtAddr::new(USER_STACK_START + 4096),
        page_table: pt,
    });
}

pub fn elf_flags_to_page_table_flags(flags: u32) -> PageTableFlags
{
    let mut res = PageTableFlags::PRESENT | PageTableFlags::USER_ACCESSIBLE;
    if flags & 1 == 0 { res |= PageTableFlags::NO_EXECUTE }
    if flags & 2 != 0 { res |= PageTableFlags::WRITABLE   }
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
    pub started: bool,

    pub ctx: Context,

    pub start_instr: VirtAddr,
    pub stack_end:   VirtAddr,
    pub page_table:  PhysAddr,
}

impl Drop for Task
{
    fn drop(self: &mut Self) {}  // TODO
}

#[derive(Default)]
pub struct Scheduler
{
    tasks: Mutex<Vec<Task>>,
    cur_task: Mutex<usize>,
}

lazy_static! {
    pub static ref SCHEDULER: Scheduler = Scheduler::new();
}

impl Scheduler
{
    pub fn new() -> Self
    {
        return Scheduler::default();
    }

    pub fn schedule_task(&self, task: Task)
    {
        self.tasks.lock().push(task);
    }

    pub unsafe fn save_current_context(&self, ctx: *const Context)
    {
        let ctx_ref = unsafe { &*ctx };

        let mut tasks = self.tasks.lock();
        let cur_task = *self.cur_task.lock();

        if let Some(task) = tasks.get_mut(cur_task) {
            task.started = true;
            task.ctx = ctx_ref.clone(); // Assuming Context implements Clone
        }
    }

    pub unsafe fn run_next_task(&self)
    {
        let tasks_len = self.tasks.lock().len();
        if tasks_len <= 0 { println!("No more tasks to run!"); crate::hlt_loop(); }

        let (started, ctx, start_instr, stack_end, page_table) = {
            let mut cur_task = self.cur_task.lock();
            let next_task = (*cur_task + 1) % tasks_len;
            *cur_task = next_task;
            let task = &self.tasks.lock()[next_task];

            (
                task.started,
                task.ctx.clone(),
                task.start_instr.clone(),
                task.stack_end.clone(),
                task.page_table.clone()
            )
        };

        unsafe { memory::activate_page_table(page_table) };

        if !started {
            unsafe { init_and_jump_to_usercode(start_instr, stack_end) };
        } else {
            unsafe { restore_context_and_return_from_interrupt(&ctx) };
        }
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

#[inline(never)]
pub unsafe fn init_and_jump_to_usercode(code: VirtAddr, stack_end: VirtAddr)
{
    unsafe
    {
        let (cs_idx, ds_idx) = crate::gdt::set_usermode_segs();
        x86_64::instructions::tlb::flush_all();
        asm!("\
        push rax    // stack segment
        push rsi    // rsp
        push 0x200  // rflags (only interrupt bit set)
        push rdx    // Code segment
        push rdi    // ret to virtual addr
        iretq",
        in("rdi") code.as_u64(), in("rsi") stack_end.as_u64(), in("dx") cs_idx, in("ax") ds_idx);
    }
}

pub unsafe extern "sysv64" fn context_switch(ctx: *const Context)
{
    unsafe {
        SCHEDULER.save_current_context(ctx);
        interrupts::notify_end_of_timer_interrupt();
        SCHEDULER.run_next_task();
    }
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

pub fn print_page_flags(flags: PageTableFlags)
{
    print!("Page flags: ");

    if flags.contains(PageTableFlags::PRESENT) {
        print!("PRESENT ");
    }
    if flags.contains(PageTableFlags::WRITABLE) {
        print!("WRITABLE ");
    }
    if flags.contains(PageTableFlags::USER_ACCESSIBLE) {
        print!("USER_ACCESSIBLE ");
    }
    if flags.contains(PageTableFlags::WRITE_THROUGH) {
        print!("WRITE_THROUGH ");
    }
    if flags.contains(PageTableFlags::NO_CACHE) {
        print!("NO_CACHE ");
    }
    if flags.contains(PageTableFlags::ACCESSED) {
        print!("ACCESSED ");
    }
    if flags.contains(PageTableFlags::DIRTY) {
        print!("DIRTY ");
    }
    if flags.contains(PageTableFlags::HUGE_PAGE) {
        print!("HUGE_PAGE ");
    }
    if flags.contains(PageTableFlags::GLOBAL) {
        print!("GLOBAL ");
    }

    println!("");
}
