use std::env;
use std::process::Command;

fn main() -> std::io::Result<()> {
    let mut args: Vec<_> = env::args_os().skip(1).collect();
    if args
        .first()
        .map(|arg| arg.to_string_lossy() == "say")
        .unwrap_or(false)
    {
        args[0] = "tts".into();
    }

    let mut command = launch_command();
    command.args(args);
    run_command(command)
}

#[cfg(unix)]
fn run_command(mut command: Command) -> std::io::Result<()> {
    use std::os::unix::process::CommandExt;
    Err(command.exec())
}

#[cfg(not(unix))]
fn run_command(mut command: Command) -> std::io::Result<()> {
    let status = command.status()?;
    std::process::exit(status.code().unwrap_or(1));
}

fn launch_command() -> Command {
    if let Ok(current_exe) = env::current_exe() {
        if let Some(dir) = current_exe.parent() {
            let sibling = dir.join("launch");
            if sibling.exists() {
                return Command::new(sibling);
            }
        }
    }

    Command::new("launch")
}
