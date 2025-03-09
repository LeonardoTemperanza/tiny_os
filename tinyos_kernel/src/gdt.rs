
use lazy_static::lazy_static;
use x86_64::structures::gdt::{Descriptor, GlobalDescriptorTable, SegmentSelector };
use x86_64::PrivilegeLevel;
use x86_64::structures::tss::TaskStateSegment;
use x86_64::VirtAddr;

pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;

lazy_static!
{
    static ref TSS: TaskStateSegment =
    {
        let mut tss = TaskStateSegment::new();
        tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] =
        {
            const STACK_SIZE: usize = 4096 * 5;
            static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];

            let stack_start = VirtAddr::from_ptr(&raw const STACK);
            let stack_end = stack_start + STACK_SIZE;
            stack_end
        };

        return tss;
    };
}

lazy_static!
{
    static ref GDT: (GlobalDescriptorTable, Selectors) =
    {
        let mut gdt = GlobalDescriptorTable::new();
        let code_selector = gdt.add_entry(Descriptor::kernel_code_segment());
        let tss_selector = gdt.add_entry(Descriptor::tss_segment(&TSS));
        let data_selector = gdt.add_entry(Descriptor::kernel_data_segment());
        return (
            gdt,
            Selectors {
                code_selector,
                tss_selector,
                data_selector
            },
        );
    };
}

struct Selectors
{
    code_selector: SegmentSelector,
    tss_selector:  SegmentSelector,
    data_selector: SegmentSelector
}

pub fn init()
{
    use x86_64::instructions::segmentation::{Segment, CS, DS};
    use x86_64::instructions::tables::load_tss;

    GDT.0.load();
    unsafe
    {
        CS::set_reg(GDT.1.code_selector);
        load_tss(GDT.1.tss_selector);
        DS::set_reg(GDT.1.data_selector);
    }
}

#[inline(always)]
pub unsafe fn set_usermode_segs() -> (u16, u16)
{
    let (mut cs, mut ds) = (GDT.1.code_selector, GDT.1.data_selector);
    cs.0 |= PrivilegeLevel::Ring3 as u16;
    ds.0 |= PrivilegeLevel::Ring3 as u16;

    use x86_64::instructions::segmentation::{Segment, CS, DS};
    unsafe { DS::set_reg(ds) };
    return (cs.0, ds.0);
}
