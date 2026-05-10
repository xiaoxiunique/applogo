use std::env;
use std::os::unix::process::CommandExt;
use std::process::Command;

fn main() -> std::io::Result<()> {
    let mut command = Command::new("launch");
    command.args(env::args_os().skip(1));
    Err(command.exec())
}
