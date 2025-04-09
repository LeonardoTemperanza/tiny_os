
use core::arch::asm;

#[inline(never)]
pub fn syscall(n: u64, arg1: u64, arg2: u64, arg3: u64, arg4: u64) -> u64
{
    let mut ret: u64;
    unsafe {
        asm!(
            "int 0x80",
            inlateout("rax") n as u64 => ret,
            in("rdi") arg1,
            in("rsi") arg2,
            in("rdx") arg3,
            in("r10") arg4,
            out("rcx") _, // rcx is used to store old rip
            out("r11") _, // r11 is used to store old rflags
            options(nostack, preserves_flags)
        );
    }

    return ret;
}

// NOTE: This should be kept up to date along with its
// counterpart in kernel code.
enum Syscall
{
    Print = 1,
    PrintNum = 2,
    PrintChar = 3,
    ReadChar = 4,
    CreateTask = 5,
    Exit = 6,
    Shutdown = 7,
}

pub fn print(string: &str)
{
    syscall(Syscall::Print as u64, string.as_ptr() as *const u8 as u64, string.len() as u64, 0, 0);
}

pub fn print_num(num: u64)
{
    syscall(Syscall::PrintNum as u64, num, 0, 0, 0);
}

pub fn print_char(c: char)
{
    syscall(Syscall::PrintChar as u64, c as u64, 0, 0, 0);
}

pub fn println(string: &str)
{
    print(string);
    print("\n");
}

pub fn exit(val: u64)
{
    syscall(Syscall::Exit as u64, val, 0, 0, 0);
}

pub fn shutdown()
{
    syscall(Syscall::Shutdown as u64, 0, 0, 0, 0);
}

pub fn create_task(task_name: &str) -> bool
{
    return unsafe { core::mem::transmute(syscall(Syscall::CreateTask as u64, task_name.as_ptr() as *const u8 as u64, task_name.len() as u64, 0, 0) as u8) };
}

pub fn read_char() -> char
{
    loop
    {
        let res = syscall(Syscall::ReadChar as u64, 0, 0, 0, 0);
        if res != 0 { return res as u8 as char; }
    }
}

pub fn read_next_line(buffer: &mut [u8]) -> &str
{
    let mut cur_idx = 0;
    loop
    {
        let next_char = read_char() as u8;
        if next_char == 0 { continue; }
        if next_char == '\n' as u8 { break; }

        cur_idx += 1;
    }

    if let Ok(input) = core::str::from_utf8(buffer) {
        return input;
    } else {
        return "";
    }
}
