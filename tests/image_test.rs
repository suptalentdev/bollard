#![type_length_limit = "2097152"]

use futures_util::future::ready;
use futures_util::stream::{StreamExt, TryStreamExt};
use tokio::runtime::Runtime;

use bollard::container::{
    Config, CreateContainerOptions, RemoveContainerOptions, StartContainerOptions,
    WaitContainerOptions,
};
use bollard::errors::Error;
use bollard::image::*;
use bollard::Docker;

use std::collections::HashMap;
use std::default::Default;
use std::fs::{remove_file, File};
use std::io::Write;

#[macro_use]
pub mod common;
use crate::common::*;

async fn create_image_test(docker: Docker) -> Result<(), Error> {
    create_image_hello_world(&docker).await?;

    Ok(())
}

async fn search_images_test(docker: Docker) -> Result<(), Error> {
    let result = &docker
        .search_images(SearchImagesOptions {
            term: "hello-world",
            ..Default::default()
        })
        .await?;

    assert!(result
        .into_iter()
        .any(|api_image| &api_image.name.as_ref().unwrap()[..] == "hello-world"));

    Ok::<_, Error>(())
}

async fn inspect_image_test(docker: Docker) -> Result<(), Error> {
    let image = if cfg!(windows) {
        format!("{}hello-world:nanoserver", registry_http_addr())
    } else {
        format!("{}hello-world:linux", registry_http_addr())
    };

    create_image_hello_world(&docker).await?;

    let result = &docker.inspect_image(&image).await?;

    assert!(result
        .repo_tags
        .as_ref()
        .unwrap()
        .iter()
        .any(|repo_tag| repo_tag == &image));

    Ok(())
}

async fn list_images_test(docker: Docker) -> Result<(), Error> {
    let image = if cfg!(windows) {
        format!("{}hello-world:nanoserver", registry_http_addr())
    } else {
        format!("{}hello-world:linux", registry_http_addr())
    };

    create_image_hello_world(&docker).await?;

    let result = &docker
        .list_images(Some(ListImagesOptions::<String> {
            all: true,
            ..Default::default()
        }))
        .await?;

    assert!(result.into_iter().any(|api_image| {
        api_image
            .repo_tags
            .iter()
            .any(|repo_tag| repo_tag == &image)
    }));

    Ok(())
}

async fn image_history_test(docker: Docker) -> Result<(), Error> {
    let image = if cfg!(windows) {
        format!("{}hello-world:nanoserver", registry_http_addr())
    } else {
        format!("{}hello-world:linux", registry_http_addr())
    };

    create_image_hello_world(&docker).await?;

    let result = &docker.image_history(&image).await?;

    assert!(result
        .iter()
        .take(1)
        .any(|history| history.tags.iter().any(|tag| tag == &image)));

    Ok(())
}

async fn prune_images_test(docker: Docker) -> Result<(), Error> {
    let mut filters = HashMap::new();
    filters.insert("label", vec!["maintainer=some_maintainer"]);
    &docker
        .prune_images(Some(PruneImagesOptions { filters }))
        .await?;

    Ok(())
}

async fn remove_image_test(docker: Docker) -> Result<(), Error> {
    let image = if cfg!(windows) {
        format!("{}hello-world:nanoserver", registry_http_addr())
    } else {
        format!("{}hello-world:linux", registry_http_addr())
    };

    create_image_hello_world(&docker).await?;

    let result = &docker
        .remove_image(
            &image,
            Some(RemoveImageOptions {
                noprune: true,
                ..Default::default()
            }),
            if cfg!(windows) {
                None
            } else {
                Some(integration_test_registry_credentials())
            },
        )
        .await?;

    assert!(result.iter().any(|s| s
        .untagged
        .as_ref()
        .map(|untagged| untagged == &image)
        .unwrap_or(false)));

    Ok(())
}

async fn commit_container_test(docker: Docker) -> Result<(), Error> {
    let image = if cfg!(windows) {
        format!("{}microsoft/nanoserver", registry_http_addr())
    } else {
        format!("{}alpine", registry_http_addr())
    };

    let cmd = if cfg!(windows) {
        Some(vec!["cmd.exe", "/C", "copy", "nul", "bollard.txt"])
    } else {
        Some(vec!["touch", "/bollard.txt"])
    };

    create_image_hello_world(&docker).await?;

    &docker
        .create_container(
            Some(CreateContainerOptions {
                name: "integration_test_commit_container",
            }),
            Config {
                cmd,
                image: Some(&image[..]),
                ..Default::default()
            },
        )
        .await?;

    &docker
        .start_container(
            "integration_test_commit_container",
            None::<StartContainerOptions<String>>,
        )
        .await?;

    &docker
        .wait_container(
            "integration_test_commit_container",
            None::<WaitContainerOptions<String>>,
        )
        .try_collect::<Vec<_>>()
        .await?;

    &docker
        .commit_container(
            CommitContainerOptions {
                container: "integration_test_commit_container",
                repo: "integration_test_commit_container_next",
                pause: true,
                ..Default::default()
            },
            Config::<String> {
                ..Default::default()
            },
        )
        .await?;

    &docker
        .create_container(
            Some(CreateContainerOptions {
                name: "integration_test_commit_container_next",
            }),
            Config {
                image: Some("integration_test_commit_container_next"),
                cmd: if cfg!(windows) {
                    Some(vec!["cmd.exe", "/C", "dir", "bollard.txt"])
                } else {
                    Some(vec!["ls", "/bollard.txt"])
                },
                ..Default::default()
            },
        )
        .await?;

    &docker
        .start_container(
            "integration_test_commit_container_next",
            None::<StartContainerOptions<String>>,
        )
        .await?;

    let vec = &docker
        .wait_container(
            "integration_test_commit_container_next",
            None::<WaitContainerOptions<String>>,
        )
        .try_collect::<Vec<_>>()
        .await?;

    let first = vec.get(0).unwrap();
    if let Some(error) = &first.error {
        println!("{}", error.message.as_ref().unwrap());
    }
    assert_eq!(first.status_code, 0);

    &docker
        .remove_container(
            "integration_test_commit_container_next",
            None::<RemoveContainerOptions>,
        )
        .await?;

    &docker
        .remove_image(
            "integration_test_commit_container_next",
            None::<RemoveImageOptions>,
            if cfg!(windows) {
                None
            } else {
                Some(integration_test_registry_credentials())
            },
        )
        .await?;

    &docker
        .remove_container(
            "integration_test_commit_container",
            None::<RemoveContainerOptions>,
        )
        .await?;

    Ok(())
}

async fn build_image_test(docker: Docker) -> Result<(), Error> {
    let dockerfile = if cfg!(windows) {
        format!(
            "FROM {}microsoft/nanoserver
RUN cmd.exe /C copy nul bollard.txt
",
            registry_http_addr()
        )
    } else {
        format!(
            "FROM {}alpine
RUN touch bollard.txt
",
            registry_http_addr()
        )
    };
    let mut header = tar::Header::new_gnu();
    header.set_path("Dockerfile").unwrap();
    header.set_size(dockerfile.len() as u64);
    header.set_mode(0o755);
    header.set_cksum();
    let mut tar = tar::Builder::new(Vec::new());
    tar.append(&header, dockerfile.as_bytes()).unwrap();

    let uncompressed = tar.into_inner().unwrap();
    let mut c = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    c.write_all(&uncompressed).unwrap();
    let compressed = c.finish().unwrap();

    let mut creds = HashMap::new();
    creds.insert(
        "localhost:5000".to_string(),
        integration_test_registry_credentials(),
    );

    &docker
        .build_image(
            BuildImageOptions {
                dockerfile: "Dockerfile".to_string(),
                t: "integration_test_build_image".to_string(),
                pull: true,
                rm: true,
                ..Default::default()
            },
            if cfg!(windows) { None } else { Some(creds) },
            Some(compressed.into()),
        )
        .try_collect::<Vec<_>>()
        .await?;

    &docker
        .create_container(
            Some(CreateContainerOptions {
                name: "integration_test_build_image",
            }),
            Config {
                image: Some("integration_test_build_image"),
                cmd: if cfg!(windows) {
                    Some(vec!["cmd.exe", "/C", "dir", "bollard.txt"])
                } else {
                    Some(vec!["ls", "/bollard.txt"])
                },
                ..Default::default()
            },
        )
        .await?;

    &docker
        .start_container(
            "integration_test_build_image",
            None::<StartContainerOptions<String>>,
        )
        .await?;

    let vec = &docker
        .wait_container(
            "integration_test_build_image",
            None::<WaitContainerOptions<String>>,
        )
        .try_collect::<Vec<_>>()
        .await?;

    let first = vec.get(0).unwrap();
    if let Some(error) = &first.error {
        println!("{}", error.message.as_ref().unwrap());
    }
    assert_eq!(first.status_code, 0);
    &docker
        .remove_container("integration_test_build_image", None)
        .await?;

    &docker
        .remove_image(
            "integration_test_build_image",
            None::<RemoveImageOptions>,
            if cfg!(windows) {
                None
            } else {
                Some(integration_test_registry_credentials())
            },
        )
        .await?;

    Ok(())
}

async fn export_image_test(docker: Docker) -> Result<(), Error> {
    create_image_hello_world(&docker).await?;

    let image = if cfg!(windows) {
        format!("{}hello-world:nanoserver", registry_http_addr())
    } else {
        format!("{}hello-world:linux", registry_http_addr())
    };
    let temp_file = if cfg!(windows) {
        "C:\\Users\\appveyor\\Appdata\\Local\\Temp\\bollard_test_image_export.tar"
    } else {
        "/tmp/bollard_test_image_export.tar"
    };

    let res = docker.export_image(&image);

    let mut archive_file = File::create(temp_file).unwrap();
    // Shouldn't load the whole file into memory, stream it to disk instead
    res.for_each(move |data| {
        archive_file.write_all(&data.unwrap()).unwrap();
        archive_file.sync_all().unwrap();
        ready(())
    })
    .await;

    // assert that the file containg the exported archive actually exists
    let test_file = File::open(temp_file).unwrap();
    // and metadata can be read
    test_file.metadata().unwrap();

    // And delete it to clean up
    remove_file(temp_file).unwrap();
    Ok(())
}

async fn issue_55_test(docker: Docker) -> Result<(), Error> {
    let dockerfile = "FROM ubuntu:18.04
RUN apt-get update && \
    apt-get install -y \
        build-essential \
        cmake \
        curl \
        file \
        git \
        graphviz \
        musl-dev \
        musl-tools \
        libpq-dev \
        libsqlite-dev \
        libssl-dev \
        linux-libc-dev \
        pkgconf \
        sudo \
        xutils-dev \
        gcc-multilib-arm-linux-gnueabihf \
        && \
    apt-get clean && rm -rf /var/lib/apt/lists/* 
";
    let mut header = tar::Header::new_gnu();
    header.set_path("Dockerfile").unwrap();
    header.set_size(dockerfile.len() as u64);
    header.set_mode(0o755);
    header.set_cksum();
    let mut tar = tar::Builder::new(Vec::new());
    tar.append(&header, dockerfile.as_bytes()).unwrap();

    let uncompressed = tar.into_inner().unwrap();
    let mut c = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    c.write_all(&uncompressed).unwrap();
    let compressed = c.finish().unwrap();

    let mut stream = docker.build_image(
        BuildImageOptions {
            dockerfile: "Dockerfile".to_string(),
            t: "issue_55".to_string(),
            pull: true,
            rm: true,
            ..Default::default()
        },
        None,
        Some(compressed.into()),
    );

    while let Some(update) = stream.next().await {
        assert!(update.is_ok());
    }

    Ok(())
}

async fn import_image_test(docker: Docker) -> Result<(), Error> {
    // round-trip test
    create_image_hello_world(&docker).await?;

    let image = if cfg!(windows) {
        format!("{}hello-world:nanoserver", registry_http_addr())
    } else {
        format!("{}hello-world:linux", registry_http_addr())
    };
    let temp_file = if cfg!(windows) {
        "C:\\Users\\appveyor\\Appdata\\Local\\Temp\\bollard_test_import_image.tar"
    } else {
        "/tmp/bollard_test_import_image.tar"
    };

    let mut res = docker.export_image(&image);
    use tokio::io::AsyncWriteExt;
    use tokio_util::codec;

    let mut archive_file = tokio::fs::File::create(temp_file).await?;
    while let Some(data) = res.next().await {
        archive_file.write_all(&data.unwrap()).await?;
        archive_file.sync_all().await?;
    }
    drop(archive_file);

    let archive_file = tokio::fs::File::open(temp_file).await?;
    let byte_stream = codec::FramedRead::new(archive_file, codec::BytesCodec::new()).map(|r| {
        let bytes = r.unwrap().freeze();
        Ok::<_, Error>(bytes)
    });

    let body = hyper::Body::wrap_stream(byte_stream);

    let mut creds = HashMap::new();
    creds.insert(
        "localhost:5000".to_string(),
        integration_test_registry_credentials(),
    );

    docker
        .import_image(
            ImportImageOptions {
                ..Default::default()
            },
            body,
            Some(creds),
        )
        .try_collect::<Vec<_>>()
        .await?;

    Ok(())
}
// ND - Test sometimes hangs on appveyor.
#[cfg(not(windows))]
#[test]
fn integration_test_search_images() {
    connect_to_docker_and_run!(search_images_test);
}

#[test]
fn integration_test_create_image() {
    connect_to_docker_and_run!(create_image_test);
}

#[test]
fn integration_test_inspect_image() {
    connect_to_docker_and_run!(inspect_image_test);
}

#[test]
fn integration_test_images_list() {
    connect_to_docker_and_run!(list_images_test);
}

#[test]
fn integration_test_image_history() {
    connect_to_docker_and_run!(image_history_test);
}

#[test]
fn integration_test_prune_images() {
    connect_to_docker_and_run!(prune_images_test);
}

#[test]
fn integration_test_remove_image() {
    connect_to_docker_and_run!(remove_image_test);
}

#[test]
fn integration_test_commit_container() {
    connect_to_docker_and_run!(commit_container_test);
}

#[test]
fn integration_test_build_image() {
    connect_to_docker_and_run!(build_image_test);
}

#[test]
#[cfg(unix)]
fn integration_test_export_image() {
    connect_to_docker_and_run!(export_image_test);
}

#[test]
#[cfg(unix)]
fn integration_test_issue_55() {
    connect_to_docker_and_run!(issue_55_test);
}

#[test]
#[cfg(unix)]
fn integration_test_import_image() {
    connect_to_docker_and_run!(import_image_test);
}
