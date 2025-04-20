
use crate::{gdt, hlt_loop, print, println, process, interrupts, memory};
use lazy_static::lazy_static;
use pic8259::ChainedPics;
use spin;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode, HandlerFunc};
use x86_64::PrivilegeLevel;
use core::arch::{naked_asm, asm};
use alloc::{vec::Vec};

pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex
{
    Timer = PIC_1_OFFSET,
    Keyboard,
    Syscall = 0x80,
}

impl InterruptIndex
{
    fn as_u8(self) -> u8 {
        self as u8
    }

    fn as_usize(self) -> usize {
        usize::from(self.as_u8())
    }
}

pub static PICS: spin::Mutex<ChainedPics> =
    spin::Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });

lazy_static!
{
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        idt.page_fault.set_handler_fn(page_fault_handler);
        unsafe {
            idt.double_fault
                .set_handler_fn(double_fault_handler)
                .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);
        }

        unsafe {
            idt[InterruptIndex::Timer.as_usize()].set_handler_fn(
                core::mem::transmute::<
                    extern "sysv64" fn(),
                    extern "x86-interrupt" fn(InterruptStackFrame)
                >(timer_interrupt_context_switch_handler)
            );

            idt[InterruptIndex::Syscall.as_usize()].set_handler_fn(
                core::mem::transmute::<
                    extern "sysv64" fn(),
                    extern "x86-interrupt" fn(InterruptStackFrame)
                >(syscall_interrupt_handler)
            ).set_privilege_level(PrivilegeLevel::Ring3);
        }

        //idt[InterruptIndex::Timer.as_usize()].set_handler_fn(timer_interrupt_handler);
        idt[InterruptIndex::Keyboard.as_usize()].set_handler_fn(keyboard_interrupt_handler);
        idt
    };
}

pub fn init_idt()
{
    IDT.load();
}

pub unsafe fn notify_end_of_timer_interrupt()
{
    unsafe { PICS.lock().notify_end_of_interrupt(InterruptIndex::Timer.as_u8()) };
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame)
{
    println!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn page_fault_handler(stack_frame: InterruptStackFrame,
                                             error_code: PageFaultErrorCode)
{
    use x86_64::registers::control::Cr2;

    println!("EXCEPTION: PAGE FAULT");
    println!("Accessed Address: {:?}", Cr2::read());
    println!("Error Code: {:?}", error_code);
    println!("{:#?}", stack_frame);
    hlt_loop();
}

extern "x86-interrupt" fn double_fault_handler(stack_frame: InterruptStackFrame, _error_code: u64) -> !
{
    panic!("EXCEPTION: DOUBLE FAULT\n{:#?}", stack_frame);
}

#[naked]
extern "sysv64" fn timer_interrupt_context_switch_handler()
{
    unsafe
    {
        naked_asm!("\
        // Save the current context
        push r15; push r14; push r13; push r12; push r11; push r10; push r9;\
        push r8; push rdi; push rsi; push rdx; push rcx; push rbx; push rax; push rbp;\
        mov rdi, rsp          // Pass the context ptr as first argument (stack array)
        sub rsp, 0x400        // Allocate some stack space
        jmp {context_switch}
        ", context_switch = sym context_switch);
    }
}

pub unsafe extern "sysv64" fn context_switch(ctx: *const process::Context)
{
    unsafe {
        process::SCHEDULER.save_current_context(ctx);
        interrupts::notify_end_of_timer_interrupt();
        process::SCHEDULER.run_next_task();
    }
}

#[naked]
extern "sysv64" fn syscall_interrupt_handler()
{
    unsafe {
        naked_asm!("\
        push rcx        // backup registers for sysretq
        push r11
        push rbp        // save callee-saved registers
        push rbx
        push r12
        push r13
        push r14
        push r15
        mov rbp, rsp    // save rsp
        sub rsp, 0x400  // make some room in the stack
        mov rcx, r10    // move fourth syscall arg to rcx which is the fourth argument register in sysv64
        mov r8, rax     // move syscall number to the 5th argument register
        call {syscall_alloc_stack} // call the handler with the syscall number in r8
        mov rsp, rbp    // restore rsp from rbp
        pop r15         // restore callee-saved registers
        pop r14
        pop r13
        pop r12
        pop rbx
        pop rbp         // restore stack and registers for sysretq
        pop r11
        pop rcx
        iretq           // use the specialized instruction to return from the interrupt (to user mode)",
        syscall_alloc_stack = sym syscall_alloc_stack);
    }
}

unsafe extern "sysv64" fn syscall_alloc_stack(arg0: u64, arg1: u64, arg2: u64, arg3: u64, syscall: u64) -> u64
{
    //println!("syscall: {}, {}, {}, {}, {}", arg0, arg1, arg2, arg3, syscall);

    /*
    let syscall_stack: Vec<u8> = Vec::with_capacity(0x10000);
    let stack_ptr = syscall_stack.as_ptr();
    */
    let retval = handle_syscall_with_temp_stack(arg0, arg1, arg2, arg3, syscall);
    //drop(syscall_stack);

    unsafe
    {
        PICS.lock().notify_end_of_interrupt(InterruptIndex::Syscall.as_u8());
    }
    return retval;
}

lazy_static! {
    pub static ref STDIN: spin::Mutex<Vec::<u8>> = spin::Mutex::new(Vec::new());
}

extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame)
{
    use pc_keyboard::{layouts, DecodedKey, HandleControl, Keyboard, ScancodeSet1};
    use spin::Mutex;
    use x86_64::instructions::port::Port;

    lazy_static!
    {
        static ref KEYBOARD: Mutex<Keyboard<layouts::Us104Key, ScancodeSet1>> =
            Mutex::new(Keyboard::new(
                ScancodeSet1::new(),
                layouts::Us104Key,
                HandleControl::Ignore
            ));
    }

    let mut keyboard = KEYBOARD.lock();
    let mut port = Port::new(0x60);

    let scancode: u8 = unsafe { port.read() };
    if let Ok(Some(key_event)) = keyboard.add_byte(scancode)
    {
        if let Some(key) = keyboard.process_keyevent(key_event)
        {
            match key {
                DecodedKey::Unicode(character) => STDIN.lock().push(character as u8),
                DecodedKey::RawKey(key) => {},
            }
        }
    }

    unsafe
    {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Keyboard.as_u8());
    }
}

// Syscalls

// NOTE: This should be kept up to date along with its
// counterpart in the usercode library.
pub enum Syscall
{
    Print = 1,
    PrintNum = 2,
    PrintChar = 3,
    ReadChar = 4,
    CreateTask = 5,
    GetArg0 = 6,
    Exit = 7,
    Shutdown = 8,
}

#[inline(never)]
extern "sysv64" fn handle_syscall_with_temp_stack(arg0: u64, arg1: u64, arg2: u64, arg3: u64, syscall: u64) -> u64
{
/*    let old_stack: *const u8;
    unsafe {
        asm!("\
        cli
        mov {old_stack}, rsp
        mov rsp, {temp_stack} // move our stack to the newly allocated one
        //sti // enable interrupts",
        temp_stack = in(reg) temp_stack, old_stack = out(reg) old_stack);
    }

*/

    let retval: u64 = match syscall
    {
        x if x == Syscall::Print as u64 => sys_print(arg0, arg1),
        x if x == Syscall::PrintNum as u64 => sys_print_num(arg0),
        x if x == Syscall::PrintChar as u64 => sys_print_char(arg0),
        x if x == Syscall::ReadChar as u64 => sys_read_char(),
        x if x == Syscall::CreateTask as u64 => sys_create_task(arg0, arg1),
        x if x == Syscall::GetArg0 as u64 => sys_get_arg_0(),
        x if x == Syscall::Exit as u64 => sys_exit(arg0),
        x if x == Syscall::Shutdown as u64 => sys_shutdown(),
        //0x1338 => sys_getline(arg0, arg1),
        //0x8EAD => sys_read(arg0, arg1, arg2),
        _ => syscall_unhandled(),
    };

/*
    unsafe {
        asm!("\
        //cli // disable interrupts while restoring the stack
        mov rsp, {old_stack} // restore the old stack
        sti
        ",
        old_stack = in(reg) old_stack);
    }
*/

    unsafe
    {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Syscall.as_u8());
    }

    return retval;
}

// TODO: For all of these usermode functions,
// for security we should sanitize all user arguments
// to make sure that they're actually userspace addresses

fn sys_print(str_ptr: u64, str_len: u64) -> u64
{

    unsafe {
        let string = core::slice::from_raw_parts(str_ptr as *const u8, str_len as usize);
        let string = core::str::from_utf8_unchecked(string);
        print!("{}", string);
    }

    return 0;
}

fn sys_print_num(val: u64) -> u64
{
    print!("{}", val);
    return 0;
}

fn sys_print_char(c: u64) -> u64
{
    if let Some(c) = char::from_u32(c as u32) {
        print!("{}", c);
        return 0;
    } else {
        return 1;
    }
}

fn sys_read_char() -> u64
{
    let mut stdin = STDIN.lock();
    if stdin.len() <= 0 { return 0; }
    return stdin.remove(0) as u64;
}

fn sys_create_task(task_name_ptr: u64, task_name_len: u64) -> u64
{
    unsafe {
        let string = core::slice::from_raw_parts(task_name_ptr as *const u8, task_name_len as usize);
        let string = core::str::from_utf8_unchecked(string);

        if string == "shell"
        {
            let kern_mem_info = memory::KERNEL_MEM_INFO.lock();
            let task = process::create_task(process::USER_PROGRAM_SHELL, kern_mem_info.phys_offset, kern_mem_info.kernel_page_table_phys_addr, 0);
            process::SCHEDULER.schedule_task(task.unwrap());
            return 1;
        }
        else if string == "rec_fib"
        {
            let kern_mem_info = memory::KERNEL_MEM_INFO.lock();
            let task = process::create_task(process::USER_PROGRAM_SHELL, kern_mem_info.phys_offset, kern_mem_info.kernel_page_table_phys_addr, 1);
            process::SCHEDULER.schedule_task(task.unwrap());
            return 1;
        }
        else
        {
            return 0;
        }
    }
}

fn sys_get_arg_0() -> u64
{
    return process::SCHEDULER.get_current_task_arg0()
}

fn sys_exit(val: u64) -> u64
{
    unsafe
    {
        process::SCHEDULER.remove_current_task();
        PICS.lock().notify_end_of_interrupt(InterruptIndex::Syscall.as_u8());
        process::SCHEDULER.run_next_task();
    }

    return val;
}

fn sys_shutdown() -> u64
{
    // NOTE: Only works in QEMU.
    unsafe { x86_64::instructions::port::Port::new(0x604).write(0x2000_u16) };
    return 0;
}

fn syscall_unhandled() -> u64
{
    panic!("Unhandled syscall!");
}
