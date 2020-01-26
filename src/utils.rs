use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};

/// Wrapper for executing any commands in command line
pub fn exec_command(cmd: &str, args: Vec<&str>) -> bool {
    let mut cli_command = Command::new(cmd)
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    for line_result in BufReader::new(cli_command.stdout.as_mut().unwrap()).lines() {
        print!("{}", line_result.unwrap());
        print!("\r\n");
    }

    for line_result in BufReader::new(cli_command.stderr.as_mut().unwrap()).lines() {
        print!("{}", line_result.unwrap());
        print!("\r\n");
    }

    cli_command.wait().unwrap().success()
}
