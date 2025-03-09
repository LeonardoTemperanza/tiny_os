
use bootloader::bootinfo::{MemoryMap, MemoryRegionType};
use crate::println;
use x86_64::
{
    structures::paging::{FrameAllocator, OffsetPageTable, PageTable, PageTableFlags, PhysFrame, Size4KiB},
    PhysAddr, VirtAddr,
};

pub unsafe fn init_kernel_page_table(phys_offset: VirtAddr) -> OffsetPageTable<'static>
{
    unsafe
    {
        let level_4_table = active_level_4_table(phys_offset);
        return OffsetPageTable::new(level_4_table, phys_offset);
    }
}

pub unsafe fn active_level_4_table(phys_offset: VirtAddr) -> &'static mut PageTable
{
    use x86_64::registers::control::Cr3;

    let (level_4_table_frame, _) = Cr3::read();

    let phys = level_4_table_frame.start_address();
    let virt = phys_offset + phys.as_u64();
    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();

    return unsafe { &mut *page_table_ptr };
}

pub unsafe fn active_level_4_table_addr() -> PhysAddr
{
    use x86_64::registers::control::Cr3;

    let (level_4_table_frame, _) = Cr3::read();

    let phys = level_4_table_frame.start_address();
    return phys;
}

pub unsafe fn clone_page_table(phys_addr: PhysAddr, frame_allocator: &mut impl FrameAllocator<Size4KiB>, phys_offset: VirtAddr) -> Option<PhysAddr>
{
    return unsafe { clone_page_table_rec(phys_addr, frame_allocator, phys_offset, 4) };
}

unsafe fn clone_page_table_rec(phys_addr: PhysAddr, frame_allocator: &mut impl FrameAllocator<Size4KiB>, phys_offset: VirtAddr, level: u8) -> Option<PhysAddr>
{
    if level <= 0 || level > 4 { panic!("Invalid level"); }

    let table_virt_addr = phys_offset + phys_addr.as_u64();
    let original_table = unsafe { &*table_virt_addr.as_ptr::<PageTable>() };

    let new_table_frame = frame_allocator.allocate_frame().expect("Phys frame allocation failed");
    let new_table_phys_addr = new_table_frame.start_address();
    let new_table_virt_addr = phys_offset + new_table_phys_addr.as_u64();

    let new_table = unsafe { &mut *new_table_virt_addr.as_mut_ptr::<PageTable>() };
    new_table.zero();

    for (i, entry) in original_table.iter().enumerate()
    {
        if entry.is_unused() { continue; }
        if !entry.flags().contains(PageTableFlags::PRESENT) { continue; }

        let entry_phys_addr = entry.addr();

        if entry.flags().contains(PageTableFlags::HUGE_PAGE) || level == 1
        {
            new_table[i] = entry.clone();
        }
        else
        {
            let res = unsafe { clone_page_table_rec(entry_phys_addr, frame_allocator, phys_offset, level - 1) };
            if let Some(new_subtable_phys) = res
            {
                new_table[i].set_addr(new_subtable_phys, entry.flags());
            }
        }
    }

    return Some(new_table_phys_addr);
}

pub unsafe fn shallow_clone_page_table(phys_addr: PhysAddr, frame_allocator: &mut impl FrameAllocator<Size4KiB>, phys_offset: VirtAddr) -> PhysAddr
{
    let table_virt_addr = phys_offset + phys_addr.as_u64();
    let original_table = unsafe { &*table_virt_addr.as_ptr::<PageTable>() };

    let new_table_frame = frame_allocator.allocate_frame().expect("Phys frame allocation failed");
    let new_table_phys_addr = new_table_frame.start_address();
    let new_table_virt_addr = phys_offset + new_table_phys_addr.as_u64();

    let new_table = unsafe { &mut *new_table_virt_addr.as_mut_ptr::<PageTable>() };
    new_table.zero();

    for (i, entry) in original_table.iter().enumerate()
    {
        if entry.is_unused() { continue; }
        if !entry.flags().contains(PageTableFlags::PRESENT) { continue; }

        new_table[i] = entry.clone();
    }

    return new_table_phys_addr;
}

pub unsafe fn activate_page_table(page_table_phys: PhysAddr)
{
    use x86_64::registers::control::{Cr3, Cr3Flags};
    let frame = PhysFrame::from_start_address(page_table_phys).unwrap();
    unsafe { Cr3::write(frame, Cr3Flags::PAGE_LEVEL_CACHE_DISABLE) };
}

pub struct EmptyFrameAllocator;

unsafe impl FrameAllocator<Size4KiB> for EmptyFrameAllocator
{
    fn allocate_frame(&mut self) -> Option<PhysFrame>
    {
        return None;
    }
}

/*
pub unsafe fn enable_page_table(table: PageTable)
{
    unsafe {
        let phys_addr = table.;
    }
}
*/

pub struct BootInfoFrameAllocator
{
    memory_map: &'static MemoryMap,
    next: usize,
}

impl BootInfoFrameAllocator
{
    pub unsafe fn init(memory_map: &'static MemoryMap) -> Self
    {
        BootInfoFrameAllocator {
            memory_map,
            next: 0,
        }
    }

    fn usable_frames(&self) -> impl Iterator<Item = PhysFrame>
    {
        // get usable regions from memory map
        let regions = self.memory_map.iter();
        let usable_regions = regions.filter(|r| r.region_type == MemoryRegionType::Usable);
        // map each region to its address range
        let addr_ranges = usable_regions.map(|r| r.range.start_addr()..r.range.end_addr());
        // transform to an iterator of frame start addresses
        let frame_addresses = addr_ranges.flat_map(|r| r.step_by(4096));
        // create `PhysFrame` types from the start addresses
        frame_addresses.map(|addr| PhysFrame::containing_address(PhysAddr::new(addr)))
    }
}

unsafe impl FrameAllocator<Size4KiB> for BootInfoFrameAllocator
{
    fn allocate_frame(&mut self) -> Option<PhysFrame>
    {
        let frame = self.usable_frames().nth(self.next);
        self.next += 1;
        return frame;
    }
}
