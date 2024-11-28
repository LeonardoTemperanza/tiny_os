
use x86_64::structures::gdt::{GlobalDescriptorTable, Descriptor, SegmentSelector};
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};
use x86_64::structures::tss::{TaskStateSegment};
use x86_64::VirtAddr;
use x86_64::registers::segmentation::CS;
use crate::println;
use core::fmt;
use lazy_static::lazy_static;
use core::arch::asm;

use pic8259::ChainedPics;
use spin;
use crate::print;

pub const DOUBLE_FAULT_IST_IDX: u16 = 0;

// Global Descriptor Table
struct Selectors
{
    code_selector: SegmentSelector,
    tss_selector: SegmentSelector
}

lazy_static!
{
    static ref GDT: (GlobalDescriptorTable, Selectors) =
    {
        let mut gdt = GlobalDescriptorTable::new();
        let code_selector = gdt.add_entry(Descriptor::kernel_code_segment());
        let tss_selector  = gdt.add_entry(Descriptor::tss_segment(&TSS));
        (gdt, Selectors { code_selector, tss_selector })
    };
}

// Interrupt Descriptor Table
lazy_static!
{
    static ref IDT: InterruptDescriptorTable =
    {
        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        idt.page_fault.set_handler_fn(page_fault_handler);
        idt.general_protection_fault.set_handler_fn(general_protection_fault_handler);

        unsafe
        {
            idt.double_fault
                .set_handler_fn(double_fault_handler)
                .set_stack_index(DOUBLE_FAULT_IST_IDX);
        }

        // Hardware interrupts
        idt[InterruptIdx::Timer.as_usize()].set_handler_fn(timer_interrupt_handler);
        idt[InterruptIdx::Keyboard.as_usize()].set_handler_fn(keyboard_interrupt_handler);
        idt[InterruptIdx::Syscall.as_usize()].set_handler_fn(syscall_interrupt_handler);
        return idt;
    };
}

// Task State Segment
lazy_static!
{
    static ref TSS: TaskStateSegment =
    {
        let mut tss = TaskStateSegment::new();
        tss.interrupt_stack_table[DOUBLE_FAULT_IST_IDX as usize] =
        {
            const STACK_SIZE: usize = 4096 * 5;
            static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];

            let stack_start = VirtAddr::from_ptr(&raw const STACK);
            let stack_end = stack_start + STACK_SIZE;
            stack_end  // In x86 stacks grow downwards
        };

        return tss;
    };
}

pub fn init_gdt()
{
    use x86_64::instructions::segmentation::{Segment, CS};
    use x86_64::instructions::tables::load_tss;

    GDT.0.load();
    unsafe
    {
        CS::set_reg(GDT.1.code_selector);
        load_tss(GDT.1.tss_selector);
    }
}

pub fn init_idt()
{
    IDT.load();
}

pub fn init_pics()
{
    unsafe { PICS.lock().initialize() };
}

////////
// Interrupt handlers

extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    println!("Exception: Breakpoint\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame,
    _error_code: u64,
) -> ! {
    panic!("Exception: Double Fault\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn general_protection_fault_handler(
    stack_frame: InterruptStackFrame, error_code: u64)
{
    println!("Exception: General Protection Fault\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    print!(".");
    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIdx::Timer.as_u8());
    }
}

extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    use pc_keyboard::{layouts, DecodedKey, HandleControl, Keyboard, ScancodeSet1};
    use spin::Mutex;
    use x86_64::instructions::port::Port;

    lazy_static! {
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
    if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
        if let Some(key) = keyboard.process_keyevent(key_event) {
            match key {
                DecodedKey::Unicode(character) => print!("{}", character),
                DecodedKey::RawKey(key) => print!("{:?}", key),
            }
        }
    }

    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIdx::Keyboard.as_u8());
    }
}

extern "x86-interrupt" fn page_fault_handler(stack_frame: InterruptStackFrame, error_code: PageFaultErrorCode)
{
    use x86_64::registers::control::Cr2;

    println!("Exception: Page Fault");
    println!("Accessed Address: {:?}", Cr2::read());
    println!("Error Code: {:?}", error_code);
    println!("{:#?}", stack_frame);
    crate::halt_loop();
}

////////
// PICS

pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

pub static PICS: spin::Mutex<ChainedPics> = spin::Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIdx
{
    Syscall   = 0x80,
    Timer     = PIC_1_OFFSET,
    Keyboard,
}

impl InterruptIdx
{
    fn as_u8(self)->u8
    {
        self as u8
    }

    fn as_usize(self)->usize
    {
        usize::from(self.as_u8())
    }
}

// Syscalls

extern "x86-interrupt" fn syscall_interrupt_handler(stack_frame: InterruptStackFrame)
{
    #[repr(C)]
    struct Registers
    {
        rax: u64,
        rbx: u64,
        rcx: u64,
        rdx: u64,
        rsi: u64,
        rdi: u64
    }

    println!("Syscall handler!");

    /*
    unsafe
    {
        asm!(
            "mov {}, rax",
            "mov {}, rbx",
            "mov {}, rcx",
            "mov {}, rdx",
            "mov {}, rsi",
            "mov {}, rdi",
            inout(reg)(&mut regs) => _,
            );
    }
    */
}
