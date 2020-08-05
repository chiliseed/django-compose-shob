use crate::utils::exec_command;

pub const DOCKER_COMPOSE: &str = "docker-compose";

/// Starts containers
pub fn start(build: bool, container: Option<String>) -> bool {
    debug!("container is: {:?}", container);
    if build {
        let mut args = vec!["build", "--force-rm"];
        if let Some(service) = &container {
            args.push(service);
        } else {
            args.push("--parallel");
        }
        exec_command(DOCKER_COMPOSE, args);
    }
    debug!("container is: {:?}", container);
    let mut args = vec!["up", "-d", "--remove-orphans"];
    if let Some(service) = &container {
        debug!("starting container");
        args.push(service);
    }
    exec_command(DOCKER_COMPOSE, args)
}

/// Stops and removes all containers
pub fn stop(service: Option<String>) -> bool {
    let mut cmd_params = vec!["rm", "--stop", "--force", "-v"];
    if let Some(service_name) = &service {
        cmd_params.push(service_name);
    }
    exec_command(DOCKER_COMPOSE, cmd_params)
}

/// Restart all containers or just one
pub fn restart(all: bool, service: &str) -> bool {
    if all {
        exec_command(DOCKER_COMPOSE, vec!["restart"])
    } else {
        exec_command(DOCKER_COMPOSE, vec!["restart", service])
    }
}

/// Rebuild specific container
pub fn rebuild(service: &str) -> bool {
    if !stop(Some(service.to_string())) {
        return false;
    }
    if !build(service) {
        return false;
    }
    exec_command(
        DOCKER_COMPOSE,
        vec!["up", "-d", "--remove-orphans", service],
    )
}

/// Build specific container
pub fn build(service: &str) -> bool {
    exec_command(DOCKER_COMPOSE, vec!["build", "--force-rm", service])
}

/// Show containers status
pub fn status() -> bool {
    exec_command(DOCKER_COMPOSE, vec!["ps", "--all"])
}

/// Show logs for container
pub fn logs(service: &str, num_lines: i32, follow: bool) -> bool {
    let tail = format!("--tail={}", num_lines.clone());
    let mut args = vec!["logs", "--timestamps", &tail];
    if follow {
        args.push("--follow");
    }
    args.push(service);
    exec_command(DOCKER_COMPOSE, args)
}

/// Execute arbitrary command inside provided service container
pub fn exec(service: &str, cmd_args: Vec<String>, workdir: Option<String>) -> bool {
    let mut cmd = vec!["exec", service];
    for arg in &cmd_args {
        cmd.push(arg);
    }
    if let Some(working_dir) = &workdir {
        info!("command will be executed in directory: {}", working_dir);
        cmd.insert(1, "--workdir");
        cmd.insert(2, working_dir);
    }
    exec_command(DOCKER_COMPOSE, cmd)
}
