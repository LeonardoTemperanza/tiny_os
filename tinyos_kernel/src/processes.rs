
pub struct BinaryInfo;

pub const USER_PROGRAM: &[u8; 1000] = include_bytes!("simple_test");
pub const USER_PROGRAM_NAME: &str = "simple_test";

use crate::println;
use crate::base::BinaryParser;

// TODO: Make some enums to make the header code clearer
// TODO: Make sure that it's the case.

#[derive(Clone, Copy, Debug)]
#[repr(C, packed)]
pub struct ElfHeader
{
    magic_num: [u8; 4],      // 0x7F, ELF
    is_64_bits: u8,          // 1 = 32 bits, 2 = 64 bits
    endianness: u8,          // 1 = little_endian, 2 = big_endian
    elf_header_version: u8,
    os_abi: u8,              // Usually 0 for System V
    _unused: [u8; 8],
    bin_type: u16,           // 1 = relocatable, 2 = executable, 3 = shared, 4 = core
    isa: u16,
    elf_version: u32,        // 1
    entry_offset: u64,
    pht_offset: u64,
    sht_offset: u64,
    flags: u32,              // Architecture dependent
    elf_header_size: u16,
    pht_entry_size:  u16,
    pht_num_entries: u16,
    sht_entry_size:  u16,
    sht_num_entries: u16,
    section_index:   u16,    // What is this?
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

pub fn parse_elf_binary(blob: &[u8])->Option<BinaryInfo>
{
    println!("Started to parse elf binary!");

    let mut parser = BinaryParser::new(blob, 0, true);
    let header = parser.next::<ElfHeader>();
    if header.magic_num[0] != 0x7F ||
       header.magic_num[1] != 'E' as u8 ||
       header.magic_num[2] != 'L' as u8 ||
       header.magic_num[3] != 'F' as u8
    {
        println!("The supplied binary is not ELF.");
        return None;
    }

    if header.is_64_bits != 2
    {
        println!("32 bits format is not supported.");
        return None;
    }

    if header.endianness != 1
    {
        println!("Big endian format is not supported.");
        return None;
    }

    if header.bin_type != 2
    {
        println!("Supported binary type used. Only executables are supported.");
        return None;
    }



    return Some(BinaryInfo{});
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