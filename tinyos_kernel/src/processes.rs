
pub struct BinaryInfo;

pub const USER_PROGRAM: &[u8; 1000] = include_bytes!("simple_test");
pub const USER_PROGRAM_NAME: &str = "simple_test";

use crate::println;
use crate::base::BinaryParser;
use core::mem::{size_of};

// TODO: Make some enums to make the header code clearer
// TODO: Make sure that it's the case.

#[derive(Clone, Copy, Debug)]
#[repr(C, packed)]
pub struct ElfHeader
{
    magic_num: [u8; 4],        // 0x7F, ELF
    is_64_bits: u8,            // 1 = 32 bits, 2 = 64 bits
    endianness: u8,            // 1 = little_endian, 2 = big_endian
    elf_header_version: u8,
    os_abi: u8,                // Usually 0 for System V
    _unused: [u8; 8],
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

#[derive(Clone, Copy, Debug)]
#[repr(C, packed)]
pub struct ProgramHeader
{
    segment_type: u32,
    offset: u64,
    vaddr:  u64,
    paddr:  u64,
    size_in_file: u64,
    size_in_memory: u64,
    flags:  u64,
    alignment: u64
}

pub fn launch_process_from_elf(blob: &[u8])->Option<BinaryInfo>
{
    println!("Started to parse elf binary!");

    

    if blob.len() < size_of::<ElfHeader>()
    {
        println!("The supplied binary is not ELF.");
        return None;
    }

    let mut parser = BinaryParser::new(blob, 0, true);
    let elf_header = parser.next::<ElfHeader>();
    if elf_header.magic_num[0] != 0x7F ||
       elf_header.magic_num[1] != 'E' as u8 ||
       elf_header.magic_num[2] != 'L' as u8 ||
       elf_header.magic_num[3] != 'F' as u8
    {
        println!("The supplied binary is not ELF.");
        return None;
    }

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

    println!("PHT num entries: {:?}", elf_header.pht_num_entries);

    // Load all of them into the desired addresses
    for i in 0..elf_header.pht_num_entries
    {
        let ph_offset = elf_header.pht_offset + (i as u64) * (elf_header.pht_entry_size as u64);
        let program_header = unsafe { &*(blob.as_ptr().add(ph_offset as usize) as *const ProgramHeader) };
        if program_header.segment_type == 1
        {
            // PT_LOAD segment, load the segment into memory
            let vaddr = program_header.vaddr as *mut u8;
            println!("{:?}", vaddr);

            // TODO: Allocate this memory, in userspace, with the proper permissions


            //unsafe
            //{
                
            //}
        }
    }

    //unsafe { jump_to_entry_point(header.entry_offset) };

    return Some(BinaryInfo{});
}

unsafe fn jump_to_entry_point(entry_point: u64)
{
    let entry: extern "C" fn() = core::mem::transmute(entry_point);
    entry();
}

pub fn run_executable(binary_info: BinaryInfo)
{

}

pub fn switch_to_usermode()
{

}

pub fn switch_to_kernelmode()
{
    
}

/*
pub struct Process
{
    pub frame: *mut TrapFrame,
    pub stack: *mut u8,
    pub pid: u16,
    pub mmu_table: *mut Table,
    pub state: ProcessState,
    pub data: ProcessData,
    pub sleep_until: usize,
    pub program: *mut u8,
    pub brk: usize
}

pub struct ProcessData
{
    pub test: u8
}

pub enum ProcessState
{
    Running,
    Sleeping,
    Waiting,
    Dead,
}
*/
