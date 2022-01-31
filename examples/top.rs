//! Run top asynchronously across several docker containers
use bollard::container::{ListContainersOptions, TopOptions};
use bollard::errors::Error;
use bollard::models::*;
use bollard::Docker;

use std::collections::HashMap;
use std::default::Default;

use futures_util::future::TryFutureExt;
use futures_util::stream::FuturesUnordered;
use futures_util::stream::StreamExt;
use tokio::runtime::Runtime;

fn main() {
    env_logger::init();

    let mut rt = Runtime::new().unwrap();
    #[cfg(unix)]
    let docker = Docker::connect_with_unix_defaults().unwrap();
    #[cfg(windows)]
    let docker = Docker::connect_with_named_pipe_defaults().unwrap();

    rt.block_on(run(docker)).unwrap();
}

async fn run(docker: Docker) -> Result<(), Error> {
    let mut list_container_filters = HashMap::new();
    list_container_filters.insert("status", vec!["running"]);

    let containers = docker
        .list_containers(Some(ListContainersOptions {
            all: true,
            filters: list_container_filters,
            ..Default::default()
        }))
        .await?;

    containers
        .iter()
        .map(|container| {
            let name = container.id.as_ref().unwrap();
            docker
                .top_processes(name, Some(TopOptions { ps_args: "aux" }))
                .map_ok(move |result| {
                    Some((name.to_owned(), result))})
        })
        .collect::<FuturesUnordered<_>>()
        .fold(Ok(HashMap::new()), |hashmap, res| match (hashmap, res) {
            (Ok(mut hashmap), Ok(opt)) => {
                if let Some((name, ContainerTopResponse{ processes: Some(p), .. })) = opt {
                    hashmap.insert(name, p.get(0).unwrap().to_vec());
                }
                futures_util::future::ok::<_, Error>(hashmap)
            }
            (Err(e), _) => futures_util::future::err(e),
            (_, Err(e)) => futures_util::future::err(e),
        })
    .map_ok(|hsh| {
        println!("                                                                \tPID\tUSER\tTIME\tCOMMAND");
        for (name, result) in hsh {
            print!("{}", name);
            for mut v in result {
                if v.len() > 30 {
                    v.truncate(30);
                }
                print!("\t{}", v);
            }
            print!("\n");
        }
    }).await?;

    Ok(())
}
