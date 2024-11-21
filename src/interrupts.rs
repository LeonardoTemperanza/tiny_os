
use x86_64::structures::gdt::{GlobalDescriptorTable, Descriptor, SegmentSelector};
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};
use x86_64::structures::tss::{TaskStateSegment};
use x86_64::VirtAddr;
use x86_64::registers::segmentation::CS;
use crate::println;
use core::fmt;
use lazy_static::lazy_static;

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
        return (gdt, Selectors { code_selector, tss_selector });
    };
}

// Interrupt Descriptor Table
lazy_static!
{
    static ref IDT: InterruptDescriptorTable =
    {
        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        unsafe
        {
            idt.double_fault
                .set_handler_fn(double_fault_handler)
                .set_stack_index(DOUBLE_FAULT_IST_IDX);
        }

        idt.double_fault.set_handler_fn(double_fault_handler);

        // Hardware interrupt
        idt[InterruptIdx::Timer.as_usize()].set_handler_fn(timer_interrupt_handler);
        idt[InterruptIdx::Keyboard.as_usize()].set_handler_fn(keyboard_interrupt_handler);



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

            let stack_start = VirtAddr::from_ptr(unsafe { &STACK });
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

extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame)
{
    println!("Exception: Breakpoint\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn double_fault_handler(stack_frame: InterruptStackFrame, _error_code: u64) -> !
{
    panic!("Exception: Double Fault (error code: {})\n{:#?}", _error_code, stack_frame);
}

extern "x86-interrupt" fn keyboard_interrupt_handler(stack_frame: InterruptStackFrame)
{
    use pc_keyboard::{layouts, DecodedKey, HandleControl, Keyboard, ScancodeSet1};
    use spin::Mutex;
    use x86_64::instructions::port::Port;

    lazy_static!
    {
        static ref KEYBOARD: Mutex<Keyboard<layouts::Us104Key, ScancodeSet1>> =
            Mutex::new(Keyboard::new(ScancodeSet1::new(),
                                     layouts::Us104Key, HandleControl::Ignore));
    }

    let mut keyboard = KEYBOARD.lock();
    let mut port = Port::new(0x60);

    let scancode: u8 = unsafe { port.read() };
    if let Ok(Some(key_event)) = keyboard.add_byte(scancode)
    {
        if let Some(key) = keyboard.process_keyevent(key_event)
        {
            match key
            {
                DecodedKey::Unicode(character) => print!("{}", character),
                DecodedKey::RawKey(key) => print!("{:?}", key),
            }
        }
    }

    // The PIC needs a notifications for the end of the interrupt
    unsafe
    {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIdx::Keyboard.as_u8());
    }
}

extern "x86-interrupt" fn timer_interrupt_handler(stack_frame: InterruptStackFrame)
{
    print!(".");

    // The pic controller needs a notifications for the end of the interrupt
    unsafe
    {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIdx::Timer.as_u8());
    }
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
    Timer = PIC_1_OFFSET,
    Keyboard,
}

impl InterruptIdx
{
    fn as_u8(self)->u8
    {
        return self as u8;
    }

    fn as_usize(self)->usize
    {
        return usize::from(self.as_u8());
    }
}
