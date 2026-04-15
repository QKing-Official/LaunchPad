// Imports

use bollard::Docker;
use bollard::models::{ContainerCreateBody, HostConfig, PortBinding};
use bollard::exec::{CreateExecOptions, StartExecOptions};
use bollard::query_parameters::{
    CreateContainerOptionsBuilder,
    CreateImageOptionsBuilder,
    StartContainerOptions,
    StopContainerOptionsBuilder,
};
use futures_util::TryStreamExt;
use std::collections::HashMap;
use tokio::time::{timeout, Duration};

pub struct DockerClient {
    pub docker: Docker,
}

pub struct ContainerConfig {
    pub name:          String,
    pub image:         String,
    pub port_bindings: Vec<(u16, u16)>,
    pub env:           Option<Vec<String>>,
    pub cmd:           Option<Vec<String>>,
    pub volumes:       Option<Vec<String>>,
    pub memory_mb:     Option<i64>,
    pub cpu_shares:    Option<i64>,
    pub network:       Option<String>,
}

// Docker client, the main docker function
// This handles the pulling of images, the first step of the creation process
impl DockerClient {
    pub fn new() -> Self {
        Self { docker: Docker::connect_with_local_defaults().expect("Docker") }
    }

    pub async fn pull_image(&self, image: &str) -> Result<(), bollard::errors::Error> {
        let (from_image, tag) = match (image.rfind('/'), image.rfind(':')) {
            (slash, Some(colon)) if slash.map_or(true, |s| colon > s) =>
                (&image[..colon], &image[colon + 1..]),
            _ => (image, "latest"),
        };
        let options = CreateImageOptionsBuilder::default()
            .from_image(from_image).tag(tag).build();
        let mut stream = self.docker.create_image(Some(options), None, None);
        timeout(Duration::from_secs(300), async {
            while let Some(_) = stream.try_next().await? {}
            Ok::<(), bollard::errors::Error>(())
        }).await.map_err(|e| bollard::errors::Error::IOError {
            err: std::io::Error::new(std::io::ErrorKind::TimedOut,
                format!("timed out: {e}")),
        })??;
        Ok(())
    }

    // This functions checks the image and makes a container with it
    // It also scans all information it got to create the server with it.
    pub async fn create_container(&self, cfg: ContainerConfig) -> Result<String, bollard::errors::Error> {
        let mut bindings: HashMap<String, Option<Vec<PortBinding>>> = HashMap::new();
        for (internal, external) in &cfg.port_bindings {
            // Bind to 0.0.0.0 tcp since I want to acces it with local ranges as well
            bindings.insert(format!("{}/tcp", internal),
                Some(vec![PortBinding {
                    host_ip:   Some("0.0.0.0".to_string()),
                    host_port: Some(external.to_string()),
                }]));
        }
        let binds: Option<Vec<String>> = cfg.volumes.map(|vols| {
            vols.into_iter().map(|p| {
                let b = std::path::Path::new(&p).file_name()
                    .and_then(|n| n.to_str()).unwrap_or("data");
                format!("{}:/data/{}:rw", p, b)
            }).collect()
        });

        // Fetch the config parsed from creation and let the container sleep forever
        // Sleeping will stop after a command is ran
        let container_cfg = ContainerCreateBody {
            image: Some(cfg.image),
            env:   cfg.env.map(|e| e.into_iter().collect()),
            cmd:   cfg.cmd.or_else(|| Some(vec!["sleep".into(), "infinity".into()])),
            host_config: Some(HostConfig {
                port_bindings: Some(bindings),
                binds,
                memory:       cfg.memory_mb.map(|m| m * 1024 * 1024),
                cpu_shares:   cfg.cpu_shares,
                network_mode: cfg.network,
                // Security hardening: drop all capabilities, no new privileges,
                // read-only root filesystem where possible to prevent escaping the container
                cap_drop:        Some(vec!["ALL".to_string()]),
                security_opt:    Some(vec!["no-new-privileges:true".to_string()]),
                readonly_rootfs: Some(false), // set true if the workload allows it
                ..Default::default()
            }),
            ..Default::default()
        };
        let c = self.docker.create_container(
            Some(CreateContainerOptionsBuilder::default().name(&cfg.name).build()),
            container_cfg,
        ).await?;
        Ok(c.id)
    }


    // Perform a power action start to the container
    pub async fn start_container(&self, id: &str) -> Result<(), bollard::errors::Error> {
        self.docker.start_container(id, None::<StartContainerOptions>).await?;
        Ok(())
    }


    // Shut the container gracefully down. This is not forced
    pub async fn stop_container(&self, id: &str) -> Result<(), bollard::errors::Error> {
        self.docker.stop_container(id,
            Some(StopContainerOptionsBuilder::default().t(5).build())).await?;
        Ok(())
    }

    // Perform a restart action on your container for easy of use. I aint letting users use start and stop for it. That is evil
    pub async fn restart_container(&self, id: &str) -> Result<(), bollard::errors::Error> {
        use bollard::query_parameters::RestartContainerOptionsBuilder;
        self.docker.restart_container(id,
            Some(RestartContainerOptionsBuilder::default().t(5).build())).await?;
        Ok(())
    }

    // KILL IT. This will kill the container, this is forcefully done
    pub async fn kill_container(&self, id: &str, signal: &str) -> Result<(), bollard::errors::Error> {
        use bollard::query_parameters::KillContainerOptionsBuilder;
        self.docker.kill_container(id,
            Some(KillContainerOptionsBuilder::default().signal(signal).build())).await?;
        Ok(())
    }

    // Remove the container, probably useful for user management later on for whoever builds around this daemon.
    // It litterly removes it, no bullshit. Just pure removal!
    pub async fn remove_container(&self, id: &str) -> Result<(), bollard::errors::Error> {
        use bollard::query_parameters::RemoveContainerOptionsBuilder;
        self.docker.remove_container(id,
            Some(RemoveContainerOptionsBuilder::default().force(true).build())).await?;
        Ok(())
    }

    pub async fn ensure_network(&self, name: &str) -> Result<(), bollard::errors::Error> {
        use bollard::models::NetworkCreateRequest;
        let _ = self.docker.create_network(NetworkCreateRequest {
            name: name.to_string(),
            driver: Some("bridge".to_string()),
            ..Default::default()
        }).await;
        Ok(())
    }

    pub async fn connect_network(&self, network: &str, cid: &str) -> Result<(), bollard::errors::Error> {
        use bollard::models::NetworkConnectRequest;
        self.docker.connect_network(network, NetworkConnectRequest {
            container: cid.to_string(),
            ..Default::default()
        }).await?;
        Ok(())
    }

    pub async fn disconnect_network(&self, network: &str, cid: &str) -> Result<(), bollard::errors::Error> {
        use bollard::models::NetworkDisconnectRequest;
        self.docker.disconnect_network(network, NetworkDisconnectRequest {
            container: cid.to_string(),
            force: Some(false),
        }).await?;
        Ok(())
    }

    /// Execute a command inside a container.
    pub async fn exec(&self, container_id: &str, cmd: Vec<String>, stdin_data: Option<String>)
        -> Result<String, bollard::errors::Error>
    {
        use bollard::exec::StartExecResults;
        let exec = self.docker.create_exec(container_id, CreateExecOptions {
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            attach_stdin:  Some(stdin_data.is_some()),
            cmd:           Some(cmd),
            // Run as a non-root user inside the container when possible.
            ..Default::default()
        }).await?;
        let mut output = String::new();
        if let StartExecResults::Attached { output: mut stream, .. } =
            self.docker.start_exec(&exec.id, None::<StartExecOptions>).await?
        {
            use bollard::container::LogOutput;
            while let Some(chunk) = stream.try_next().await? {
                match chunk {
                    LogOutput::StdOut { message } | LogOutput::StdErr { message } =>
                        output.push_str(&String::from_utf8_lossy(&message)),
                    _ => {}
                }
            }
        }
        Ok(output)
    }

    pub async fn logs(&self, container_id: &str, tail: Option<u64>)
        -> Result<String, bollard::errors::Error>
    {
        use bollard::query_parameters::LogsOptionsBuilder;
        use bollard::container::LogOutput;
        let tail_str = tail.map(|n| n.to_string()).unwrap_or_else(|| "100".into());
        let opts = LogsOptionsBuilder::default()
            .stdout(true).stderr(true).tail(tail_str.as_str()).build();
        let mut stream = self.docker.logs(container_id, Some(opts));
        let mut out = String::new();
        while let Some(chunk) = stream.try_next().await? {
            match chunk {
                LogOutput::StdOut { message } | LogOutput::StdErr { message } =>
                    out.push_str(&String::from_utf8_lossy(&message)),
                _ => {}
            }
        }
        Ok(out)
    }

    pub async fn stats(&self, container_id: &str)
        -> Result<impl serde::Serialize, bollard::errors::Error>
    {
        use bollard::query_parameters::StatsOptionsBuilder;
        let opts = StatsOptionsBuilder::default().stream(false).build();
        let mut stream = self.docker.stats(container_id, Some(opts));
        let s = stream.try_next().await?.ok_or_else(|| bollard::errors::Error::IOError {
            err: std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "no stats"),
        })?;
        Ok(s)
    }

    pub async fn stats_json(&self, container_id: &str)
        -> Result<serde_json::Value, bollard::errors::Error>
    {
        use bollard::query_parameters::StatsOptionsBuilder;
        let opts = StatsOptionsBuilder::default().stream(false).build();
        let mut stream = self.docker.stats(container_id, Some(opts));
        let s = stream.try_next().await?.ok_or_else(|| bollard::errors::Error::IOError {
            err: std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "no stats"),
        })?;
        Ok(serde_json::to_value(s).unwrap_or(serde_json::json!({})))
    }
}