
use x86_64::
{
    structures::paging::*,
    structures::paging::mapper::MapToError,
    VirtAddr, PhysAddr
};

use bootloader::bootinfo::*;

use core::ptr::null_mut;
use alloc::alloc::{GlobalAlloc, Layout};

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

    fn usable_frames(&self)->impl Iterator<Item = PhysFrame>
    {
        let regions = self.memory_map.iter();
        let usable_regions = regions.filter(|r| r.region_type == MemoryRegionType::Usable);
        let addr_ranges = usable_regions.map(|r| r.range.start_addr()..r.range.end_addr());
        let frame_addresses = addr_ranges.flat_map(|r| r.step_by(4096));
        return frame_addresses.map(|addr| PhysFrame::containing_address(PhysAddr::new(addr)));
    }
}

unsafe impl FrameAllocator<Size4KiB> for BootInfoFrameAllocator
{
    fn allocate_frame(&mut self)->Option<PhysFrame>
    {
        let frame = self.usable_frames().nth(self.next);
        self.next += 1;
        return frame;
    }
}

// Dummy allocator used for global allocations

pub const HEAP_START: usize = 0x_4444_4444_0000;
pub const HEAP_SIZE: usize = 100 * 1024;

#[global_allocator]
static ALLOCATOR: linked_list_allocator::LockedHeap = linked_list_allocator::LockedHeap::empty();

pub fn init_heap(mapper: &mut impl Mapper<Size4KiB>,
                 frame_allocator: &mut impl FrameAllocator<Size4KiB>)->Result<(), MapToError<Size4KiB>>
{
    let page_range = {
        let heap_start = VirtAddr::new(HEAP_START as u64);
        let heap_end = heap_start + HEAP_SIZE - 1u64;
        let heap_start_page = Page::containing_address(heap_start);
        let heap_end_page = Page::containing_address(heap_end);
        Page::range_inclusive(heap_start_page, heap_end_page)
    };

    // Map all heap pages to physical frames
    for page in page_range
    {
        let frame = frame_allocator
            .allocate_frame()
            .ok_or(MapToError::FrameAllocationFailed)?;

        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
        unsafe
        {
            mapper.map_to(page, frame, flags, frame_allocator)?.flush()
        }
    }

    unsafe
    {
        ALLOCATOR.lock().init(HEAP_START, HEAP_SIZE);
    }

    return Ok(());
}
