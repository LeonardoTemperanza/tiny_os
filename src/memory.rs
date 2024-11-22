
use x86_64::
{
    structures::paging::*,
    VirtAddr, PhysAddr
};

use bootloader::bootinfo::*;

pub unsafe fn init(phys_mem_offset: VirtAddr)->OffsetPageTable<'static>
{
    let level_4_table = active_level_4_table(phys_mem_offset);
    return OffsetPageTable::new(level_4_table, phys_mem_offset);
}

// Unsafe because of the mutable reference and also it needs to be mapped
pub unsafe fn active_level_4_table(physical_memory_offset: VirtAddr)->&'static mut PageTable
{
    use x86_64::registers::control::Cr3;

    let (level_4_table_frame, _) = Cr3::read();

    let phys = level_4_table_frame.start_address();
    let virt = physical_memory_offset + phys.as_u64();
    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();

    return &mut *page_table_ptr;
}

pub unsafe fn translate_addr(addr: VirtAddr, physical_memory_offset: VirtAddr)->Option<PhysAddr>
{
    use x86_64::structures::paging::page_table::FrameError;
    use x86_64::registers::control::Cr3;

    // Read the active level 4 frame from the CR3 register
    let (level_4_table_frame, _) = Cr3::read();

    let table_indices = [
        addr.p4_index(), addr.p3_index(), addr.p2_index(), addr.p1_index()
    ];
    let mut frame = level_4_table_frame;

    // Traverse the multi-level page table
    for &index in &table_indices
    {
        // Convert the frame into a page table reference
        let virt = physical_memory_offset + frame.start_address().as_u64();
        let table_ptr: *const PageTable = virt.as_ptr();
        let table = unsafe {&*table_ptr};

        // Read the page table entry and update "frame"
        let entry = &table[index];
        frame = match entry.frame()
        {
            Ok(frame) => frame,
            Err(FrameError::FrameNotPresent) => return None,
            Err(FrameError::HugeFrame) => panic!("Huge pages not supported")
        };
    }

    // Calculate the physical address by adding the page offset
    return Some(frame.start_address() + u64::from(addr.page_offset()));
}

pub fn create_example_mapping(page: Page, mapper: &mut OffsetPageTable, frame_allocator: &mut impl FrameAllocator<Size4KiB>)
{
    use x86_64::structures::paging::PageTableFlags as Flags;

    let frame = PhysFrame::containing_address(PhysAddr::new(0xb80000));
    let flags = Flags::PRESENT | Flags::WRITABLE;

    let map_to_result = unsafe
    {
        mapper.map_to(page, frame, flags, frame_allocator)
    };

    map_to_result.expect("map_to failed").flush();
}

////////
// Allocators

// A FrameAllocator that always returns 'None'

pub struct EmptyFrameAllocator;

unsafe impl FrameAllocator<Size4KiB> for EmptyFrameAllocator
{
    fn allocate_frame(&mut self)->Option<PhysFrame>
    {
        return None;
    }
}

// A FrameAllocator that returns usable frames from the bootloader's memory map.
pub struct BootInfoFrameAllocator
{
    memory_map: &'static MemoryMap,
    next: usize,
}

impl BootInfoFrameAllocator
{
    pub unsafe fn init(memory_map: &'static MemoryMap)->Self
    {
        return BootInfoFrameAllocator
        {
            memory_map,
            next: 0,
        };
    }
}
