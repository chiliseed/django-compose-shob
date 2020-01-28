use std::error::Error;
use std::fmt;
use std::fs::File;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::Path;

use ssh2::Session;

use crate::utils::exec_command;

const MB: usize = 1_000_000;
const CHUNK_SIZE: usize = 1 * MB;

#[derive(Debug, PartialEq)]
pub enum DeployError {
    AuthenticationFailed(String),
    ConnectionError(String),
    SessionError(String),
    RemoteCmdError(String),
}

impl Error for DeployError {
    fn description(&self) -> &str {
        match *self {
            DeployError::AuthenticationFailed(ref cause) => cause,
            DeployError::ConnectionError(ref cause) => cause,
            DeployError::SessionError(ref cause) => cause,
            DeployError::RemoteCmdError(ref cause) => cause,
        }
    }
}

impl fmt::Display for DeployError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.description())
    }
}

fn get_session(
    server_ip: &str,
    server_user: &str,
    ssh_key: Option<String>,
) -> Result<Session, DeployError> {
    let tcp = match TcpStream::connect(format!("{}:22", server_ip)) {
        Ok(stream) => stream,
        Err(err) => return Err(DeployError::ConnectionError(err.to_string())),
    };
    let mut sess = match Session::new() {
        Ok(s) => s,
        Err(err) => return Err(DeployError::SessionError(err.to_string())),
    };
    sess.set_tcp_stream(tcp);
    match sess.handshake() {
        Ok(()) => println!("Handshake success"),
        Err(err) => return Err(DeployError::SessionError(err.to_string())),
    }

    match ssh_key {
        Some(key) => match sess.userauth_pubkey_file(server_user, None, &Path::new(&key), None) {
            Ok(()) => Ok(sess),
            Err(err) => Err(DeployError::AuthenticationFailed(err.to_string())),
        },

        None => match sess.userauth_agent(server_user) {
            Ok(()) => Ok(sess),
            Err(err) => Err(DeployError::AuthenticationFailed(err.to_string())),
        },
    }
}

fn exec_cmd_on_server(ssh_conn: &Session, cmd: &str) -> Result<i32, DeployError> {
    println!("[remote]: {}", cmd);
    let mut channel = match ssh_conn.channel_session() {
        Ok(c) => c,
        Err(err) => return Err(DeployError::SessionError(err.to_string())),
    };

    channel.exec(cmd).unwrap();
    let mut buffer = Vec::new();
    loop {
        let n = std::io::Read::by_ref(&mut channel).take(10).read_to_end(&mut buffer).unwrap();
        if n == 0 { break; }
        print!("{}", String::from_utf8_lossy(&buffer));
    }
    channel.wait_close().unwrap();
    Ok(channel.exit_status().unwrap())
}

pub fn execute(
    server_ip: &str,
    server_user: &str,
    ssh_key: Option<String>,
    deploy_dir: &str,
    excluded_patterns: Option<Vec<String>>,
) {
    let name = "deployment";
    let deployment_package = format!("{}.tar.gz", name);
    let mut tar_args = vec!["-zcf", deployment_package.as_str()];
    if let Some(excludes) = &excluded_patterns {
        for p in excludes.iter() {
            tar_args.push("--exclude");
            tar_args.push(p.as_str());
        }
    }

    tar_args.push(deploy_dir);

    if !exec_command("tar", tar_args) {
        eprintln!("Failed to gzip deploy target: {}", deploy_dir);
        return;
    }

    let ssh_conn = match get_session(server_ip, server_user, ssh_key) {
        Ok(s) => s,
        Err(err) => {
            eprintln!("{}", err);
            return;
        }
    };

    let mut deployment_package_fp = match File::open(deployment_package.clone()) {
        Ok(fp) => fp,
        Err(err) => {
            eprintln!("Failed to open deployment package: {}", err);
            return;
        }
    };
    let pck_meta = match deployment_package_fp.metadata() {
        Ok(meta_data) => meta_data,
        Err(err) => {
            eprintln!("Failed to get metadata: {}", err);
            return;
        }
    };

    let mut channel = match ssh_conn.scp_send(
        Path::new(&format!("/tmp/{}", deployment_package)),
        0o644,
        pck_meta.len(),
        None,
    ) {
        Ok(c) => c,
        Err(err) => {
            eprintln!("Failed to open a channel: {}", err);
            return;
        }
    };

    let mut buffer = Vec::new();
    loop {
        let read_bytes = match std::io::Read::by_ref(&mut deployment_package_fp)
            .take(CHUNK_SIZE as u64)
            .read_to_end(&mut buffer)
        {
            Ok(chunk) => chunk,
            Err(err) => {
                eprintln!("Error reading chunk: {}", err);
                return;
            }
        };
        if read_bytes == 0 {
            break;
        }
        match channel.write(&mut buffer) {
            Ok(_n) => println!("Uploaded chunk"),
            Err(err) => {
                eprintln!("Failed to upload a chunk: {}", err);
                return;
            }
        };
    }

    println!("Deployment packages uploaded OK");

    println!("Extracting deployment package");
    exec_cmd_on_server(
        &ssh_conn,
        format!("mkdir -p /home/{}/web", server_user).as_str(),
    )
    .unwrap();
    exec_cmd_on_server(
        &ssh_conn,
        format!("tar -zxvf /tmp/{} -C /home/{}/web", deployment_package, server_user).as_str(),
    )
    .unwrap();

    println!("Stopping existing containers");
    exec_cmd_on_server(
        &ssh_conn,
        format!("cd /home/{}/web; docker-compose rm -s -f", server_user).as_str(),
    )
    .unwrap();

    println!("Build and start services");
    exec_cmd_on_server(
        &ssh_conn,
        format!("cd /home/{}/web; docker-compose up -d --build", server_user).as_str(),
    )
    .unwrap();

    exec_cmd_on_server(
        &ssh_conn,
        format!("rm -rf /tmp/{}", deployment_package).as_str(),
    )
    .unwrap();

    exec_command("rm", vec!["-rf", deployment_package.as_str()]);
}
