use std::error::Error;
use std::fs::File;
use std::io::BufRead;
use std::io::{BufReader, Read, Write};
use std::net::TcpStream;
use std::path::Path;
use std::{fmt, fs, io};

use flate2::write::GzEncoder;
use flate2::Compression;
use globset::{Glob, GlobSetBuilder};
use ssh2::Session;
use uuid::Uuid;

use crate::utils::exec_command;
use walkdir::WalkDir;

#[derive(Debug)]
pub enum DeployError {
    AuthenticationFailed(String),
    ConnectionError(ssh2::Error),
    SessionError(String),
    RemoteCmdError(String),
    ParseError(globset::Error),
    IOError(io::Error),
}

type DeploymentResult<T> = Result<T, DeployError>;

impl Error for DeployError {}

impl fmt::Display for DeployError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            DeployError::AuthenticationFailed(ref cause) => write!(f, "{}", cause),
            DeployError::ConnectionError(ref err) => err.fmt(f),
            DeployError::SessionError(ref cause) => write!(f, "{}", cause),
            DeployError::RemoteCmdError(ref cause) => write!(f, "{}", cause),
            DeployError::ParseError(ref err) => err.fmt(f),
            DeployError::IOError(ref err) => err.fmt(f),
        }
    }
}

impl From<globset::Error> for DeployError {
    fn from(err: globset::Error) -> DeployError {
        DeployError::ParseError(err)
    }
}

impl From<ssh2::Error> for DeployError {
    fn from(err: ssh2::Error) -> DeployError {
        DeployError::ConnectionError(err)
    }
}

impl From<io::Error> for DeployError {
    fn from(err: io::Error) -> DeployError {
        DeployError::IOError(err)
    }
}

fn get_session(
    server_ip: &str,
    server_user: &str,
    ssh_key: Option<String>,
) -> DeploymentResult<Session> {
    let tcp = TcpStream::connect(format!("{}:22", server_ip))?;
    let mut sess = Session::new()?;

    sess.set_tcp_stream(tcp);
    sess.handshake()?;

    match ssh_key {
        Some(key) => {
            println!("Connecting via key: {}", key);
            match sess.userauth_pubkey_file(server_user, None, &Path::new(&key), None) {
                Ok(()) => Ok(sess),
                Err(err) => Err(DeployError::AuthenticationFailed(err.to_string())),
            }
        },

        None => match sess.userauth_agent(server_user) {
            Ok(()) => Ok(sess),
            Err(err) => Err(DeployError::AuthenticationFailed(err.to_string())),
        },
    }
}

const BUILD_LOCATION: &str = "_build";
const BUILD_ARTIFACT: &str = "build";

fn create_build_tarball() -> Result<String, DeployError> {
    let uuid = Uuid::new_v4();
    let build_tar_name = format!("build_{}.tar.gz", uuid.to_simple());
    let build_tar = File::create(build_tar_name.clone())?;
    let encoder = GzEncoder::new(build_tar, Compression::default());
    let mut tar = tar::Builder::new(encoder);
    tar.append_dir_all(BUILD_ARTIFACT, BUILD_LOCATION)?;
    Ok(build_tar_name)
}

fn upload_build_tarball_to_server(ssh_conn: &Session, build_tarball: &str) -> DeploymentResult<()> {
    println!("Uploading {} to build worker", build_tarball);
    let mut deployment_package_fp = File::open(build_tarball)?;
    let pck_meta = deployment_package_fp.metadata()?;
    let mut channel = ssh_conn.scp_send(
        Path::new(&format!("/tmp/{}", build_tarball)),
        0o644,
        pck_meta.len(),
        None,
    )?;

    loop {
        let mut buffer = Vec::new();
        let read_bytes = std::io::Read::by_ref(&mut deployment_package_fp)
            .take(1000)
            .read_to_end(&mut buffer)?;
        if read_bytes == 0 {
            break;
        }
        channel.write_all(&buffer)?;
    }

    Ok(())
}

fn setup_deployment_dir() -> DeploymentResult<()> {
    if Path::new(BUILD_LOCATION).exists() {
        println!("Removing previous artifact");
        fs::remove_dir_all(BUILD_LOCATION)?;
    }

    println!("Setting up deployment artifact");
    fs::create_dir(BUILD_LOCATION)?;

    let mut ignores: Vec<String> = vec![
        "*.pem".to_string(),
        ".git/*".to_string(),
        "_build/*".to_string(),
        "*.tar.gz".to_string(),
    ];

    match File::open(".gitignore") {
        Ok(gitignore_file) => {
            ignores = BufReader::new(gitignore_file)
                .lines()
                .filter_map(|line| line.ok())
                .filter(|line| !line.trim().is_empty())
                .collect();
        }
        Err(_) => {
            eprintln!(".gitignore not found");
        }
    };


    let mut path_checker = GlobSetBuilder::new();
    ignores.iter().for_each(|ignore_pattern| {
        let mut clean_ignore = ignore_pattern.trim().to_string();
        if clean_ignore.starts_with("/") {
            debug!("Adding .{} to ignore", clean_ignore);
            clean_ignore = ".".to_string() + &clean_ignore;
        } else if !clean_ignore.starts_with("./") {
            debug!("Adding ./{} to ignore", clean_ignore);
            clean_ignore = "./".to_string() + &clean_ignore;
        }
        if Path::new(&clean_ignore).is_dir() {
            debug!("Adding * to {} ignore", clean_ignore);
            clean_ignore = clean_ignore + "/*";
        }
        debug!("Ignoring path: {}", clean_ignore);
        path_checker.add(Glob::new(&clean_ignore).unwrap());
    });

    let set_path_checker = path_checker.build()?;

    for entry in WalkDir::new(".")
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.is_dir() {
            continue;
        }

        let matched_patterns_idx = set_path_checker.matches(path);
        if !matched_patterns_idx.is_empty() {
            continue;
        }

        let move_to = format!("{}/{}", &BUILD_LOCATION, path.to_str().unwrap());
        let build_path = Path::new(&move_to);

        fs::create_dir_all(build_path.parent().unwrap())?;
        fs::copy(path, build_path)?;
    }
    Ok(())
}

fn exec_cmd_on_server(ssh_conn: &Session, cmd: &str) -> DeploymentResult<i32> {
    println!("[remote]: {}", cmd);
    let mut channel = ssh_conn.channel_session()?;

    channel.exec(cmd).unwrap();
    loop {
        let mut buffer = Vec::new();
        let n = std::io::Read::by_ref(&mut channel)
            .take(10)
            .read_to_end(&mut buffer)
            .unwrap();
        if n == 0 {
            let mut s = String::new();
            channel.stderr().read_to_string(&mut s).unwrap();
            eprintln!("{}", s);
            break;
        }
        print!("{}", String::from_utf8_lossy(&buffer));
    }
    channel.wait_close().unwrap();
    Ok(channel.exit_status().unwrap())
}

pub fn execute(server_ip: &str, server_user: &str, ssh_key: Option<String>) {
    // prepare build directory
    match setup_deployment_dir() {
        Ok(()) => debug!("deployment dir is ready"),
        Err(err) => {
            eprintln!("Error: {}", err);
            return;
        }
    }

    // create tar.gz build directory
    let build_tarball = match create_build_tarball() {
        Ok(tarball) => {
            println!("Build tarballed ok");
            tarball
        }
        Err(err) => {
            eprintln!("Error: {}", err);
            return;
        }
    };

    let ssh_conn = match get_session(server_ip, server_user, ssh_key) {
        Ok(s) => s,
        Err(err) => {
            eprintln!("{}", err);
            return;
        }
    };

    // upload tar.gz to worker server
    match upload_build_tarball_to_server(&ssh_conn, &build_tarball) {
        Ok(()) => println!("Build uploaded to server"),
        Err(err) => {
            eprintln!("Error: {}", err);
            return;
        }
    };
    println!("\r\nDeployment packages uploaded OK");

    println!("Clearing web directory");
    match exec_cmd_on_server(
        &ssh_conn,
        format!("rm -rf /home/{}/web", server_user).as_str(),
    ) {
        Ok(status_code) => {
            if status_code > 0 {
                eprintln!("Error. Exiting");
                return;
            }
        }
        Err(err) => {
            eprintln!("Failed to clear web directory: {}", err);
            return;
        }
    }
    println!("Extracting deployment package");
    match exec_cmd_on_server(
        &ssh_conn,
        format!("mkdir -p /home/{}/web", server_user).as_str(),
    ) {
        Ok(status_code) => {
            if status_code > 0 {
                eprintln!("Error. Exiting");
                return;
            }
        }
        Err(err) => {
            eprintln!("Failed to setup web structure: {}", err);
            return;
        }
    }
    match exec_cmd_on_server(
        &ssh_conn,
        format!(
            "tar -xzvf /tmp/{} -C /tmp",
            build_tarball
        )
        .as_str(),
    ) {
        Ok(status_code) => {
            if status_code > 0 {
                eprintln!("Error extracting build tarball. Exiting");
                return;
            }
        }
        Err(err) => {
            eprintln!("Failed to extract deployment bundle: {}", err);
            return;
        }
    }

    match exec_cmd_on_server(
        &ssh_conn,
        format!(
            "cp -r /tmp/{}/* /home/{}/web",
            BUILD_ARTIFACT, server_user
        )
        .as_str(),
    ) {
        Ok(status_code) => {
            if status_code > 0 {
                eprintln!("Error copying file to web directory. Exiting");
                return;
            }
        }
        Err(err) => {
            eprintln!("Failed to extract deployment bundle: {}", err);
            return;
        }
    }

    println!("Stopping existing containers");
    match exec_cmd_on_server(
        &ssh_conn,
        format!("cd /home/{}/web; docker-compose rm -s -f", server_user).as_str(),
    ) {
        Ok(status_code) => {
            if status_code > 0 {
                eprintln!("Error. Exiting");
                return;
            }
        }
        Err(err) => {
            eprintln!("Failed to stop docker containers: {}", err);
            return;
        }
    }

    println!("Build and start services");
    match exec_cmd_on_server(
        &ssh_conn,
        format!("cd /home/{}/web; docker-compose up -d --build", server_user).as_str(),
    ) {
        Ok(status_code) => {
            if status_code > 0 {
                eprintln!("Error. Exiting");
                return;
            }
        }
        Err(err) => {
            eprintln!("Failed to build and start the containers: {}", err);
            return;
        }
    }

    match exec_cmd_on_server(&ssh_conn, format!("rm -rf /tmp/{}", build_tarball).as_str()) {
        Ok(status_code) => {
            if status_code > 0 {
                eprintln!("Error. Exiting");
                return;
            }
        }
        Err(err) => {
            eprintln!("Failed to remove deployment package from server: {}", err);
            return;
        }
    }

    exec_command("rm", vec!["-rf", build_tarball.as_str()]);
}
