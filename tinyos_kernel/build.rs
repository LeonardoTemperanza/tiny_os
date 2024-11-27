
use std::fs;
use std::process::Command;
use std::path::{Path, PathBuf};

fn main()
{
    let userspace_dir = Path::new("../userspace");
    let programs = find_programs(userspace_dir);

}

fn find_programs(userspace_dir: &Path)->Vec<PathBuf>
{
    let mut res = Vec::new();

    if let Ok(entries) = fs::read_dir(userspace_dir)
    {
        for entry in entries.flatten()
        {
            let path = entry.path();
            if path.is_dir() && path.join("Cargo.toml").exists()
            {
                programs.push(path)
            }
        }
    }
}