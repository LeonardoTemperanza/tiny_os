
use std::process::Command;
use std::fs;
use std::env;

fn main()
{
    env::set_current_dir("user_programs").expect("Failed to change current working directory");

    let output = Command::new("rustc")
        .args([
            "--target", "../tinyos_x64_target.json",
            "-o", "user_program1",
            "user_program1.rs"
            ])
        .output()
        .expect("Failed to compile user program");

        if !output.status.success()
        {
            panic!("user program compilation failed:\n{}", String::from_utf8_lossy(&output.stderr));
        }

        fs::copy("user_program", "src/user_program").expect("Failed to copy user program binary");

        // Rebuild if the user program changes
        println!("cargo:rerun-if-changed={}", "user_program1.rs");
}