
#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(dead_code)]

use core::panic::PanicInfo;
mod tinyos_userlib;
use crate::tinyos_userlib::*;

#[panic_handler]
fn panic(info: &PanicInfo)->!
{
    println("Panicking!");
    loop {}
}

#[unsafe(no_mangle)]
pub extern "C" fn _start()
{
    let mut exit_code;
    if get_arg_0() == 0 {
        exit_code = shell_main();
    } else {
        loop {
            exit_code = rec_fib_main();
            exit(exit_code);
        }
    }

    exit(exit_code);
}

pub fn run_command(input: &str)
{
    if input == "" { return; }

    if input == "help"
    {
        println("  -HELP-");
        println("  Here's the list of available commands:");
        println("  help -- this command.");
        println("  run [task_name] -- launches a new task in parallel.");
        println("                     Task names: shell, rec_fib.");
        println("  quit_shell -- exits this process. (this will leave the scheduler empty!)");
        println("  shutdown -- turns off this PC. (only works for QEMU)");
    }
    else if input == "shutdown"
    {
        println("Shutting down system...");
        shutdown();
    }
    else if input == "quit_shell"
    {
        println("Quitting...");
        exit(0);
    }
    else
    {
        if input.starts_with("run ")
        {
            let program_name = &input[4..];
            let ok = create_task(program_name);
            if ok { println("Successfully launched the task."); }
            else  { println("Failed to create task."); }
        }
        else
        {
            println("Unrecognized command. Type 'help' for a list of available commands.");
        }
    }
}


pub fn shell_main() -> u64
{
    println("Welcome to TinyOS! I'm a user-program \"shell\".");
    println("Type 'help' for a list of available commands.");
    print("> ");

    let mut buffer: [u8; 512] = [0; 512];
    let mut buf_idx = 0;
    loop
    {

        let input_char = read_char();
        print_char(input_char);
        if input_char as u8 == 0x08 // Backspace
        {
            if buf_idx > 0 {
                buf_idx -= 1;
            }
        }
        else if input_char == '\n'
        {
            if let Ok(input_string) = core::str::from_utf8(&buffer[..buf_idx]) {
                run_command(input_string);
            }

            buf_idx = 0;
            print("> ");
        }
        else
        {
            buffer[buf_idx] = input_char as u8;

            if buf_idx < buffer.len() - 1 {
                buf_idx += 1;
            }
        }
    }
}

pub fn rec_fib_main() -> u64
{
    println("About to compute the 40th fibonacci number recursively...");
    let res = rec_fib(39);
    print("Result: ");
    print_num(res);
    println("");

    return 0;
}

pub fn rec_fib(n: u64) -> u64
{
    if n == 0 { return 0; }
    if n == 1 { return 1; }
    return rec_fib(n-1) + rec_fib(n-2);
}
