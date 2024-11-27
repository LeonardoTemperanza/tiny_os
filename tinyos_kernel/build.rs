
use std::fs;
use std::process::Command;
use std::path::{Path, PathBuf};

// This build program runs cargo to build the user programs and
// then embeds the binaries into the kernel. (This is just a replacement
// to get user processes up and running until a get a file system working.)
fn main()
{
    // Get list of programs from the /src directory in the user_programs workspace
    let programs = fs::read_dir("../user_programs/src").unwrap();

    let args = ["build",
                "--release"];

    let status = Command::new("cargo")
        .args(args)
        .current_dir("../user_programs")
        .status()
        .expect("Failed to build userspace programs");

    if !status.success() { panic!("Building userspace programs failed"); }

    let binary_path = format!("../target/tinyos_x64_target/release/");
    for entry in programs
    {
        if let Ok(entry) = entry
        {
            println!("{:?}", entry.file_name());
        }
    }

    /*
    let meta = String::from("let ");
    program_list.push_str(&format!(
                          "    UserProgram {{ name: \"{}\", data: include_bytes!(\"{}\") }},\n",
                          binary_name, binary_path));

    program_list.push_str("];\n");
    fs::write("src/embedded_programs.rs", program_list)
        .expect("Failed to write embedded programs list");

    // Rerun if anything changes
    println!("cargo:rerun-if-chagned=../userspace");
    */
}
