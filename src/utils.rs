use std::error::Error;
use std::process::{Command, Stdio};

/// Wrapper for executing any commands in command line
pub fn exec_command(cmd: &str, args: Vec<&str>) -> bool {
    println!("{}", cmd);
    let mut cli_command = match Command::new(cmd)
        .args(&args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
    {
        Err(err) => panic!("Error spawning: {}", err.description()),
        Ok(process) => process,
    };

    cli_command.wait().unwrap().success()
}
