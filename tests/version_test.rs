use bollard::system::Version;
use bollard::{ClientVersion, Docker};
use std::future::Future;
use tokio::runtime::Runtime;

#[macro_use]
mod common;

#[cfg(windows)]
#[test]
fn test_version_named_pipe() {
    rt_exec!(
        Docker::connect_with_named_pipe_defaults()
            .unwrap()
            .version(),
        |version: Version| assert_eq!(version.os, "windows")
    )
}

#[cfg(all(unix, not(feature = "test_http"), not(feature = "ssl")))]
#[test]
fn test_version_unix() {
    rt_exec!(
        Docker::connect_with_unix_defaults().unwrap().version(),
        |version: Version| assert_eq!(version.os, "linux")
    )
}

#[cfg(feature = "ssl")]
#[test]
fn test_version_ssl() {
    rt_exec!(
        Docker::connect_with_ssl_defaults().unwrap().version(),
        |version: Version| assert_eq!(version.os, "linux")
    )
}

#[cfg(feature = "test_http")]
#[test]
fn test_version_http() {
    #[cfg(unix)]
    rt_exec!(
        Docker::connect_with_http_defaults().unwrap().version(),
        |version: Version| assert_eq!(version.os, "linux")
    );
    #[cfg(windows)]
    rt_exec!(
        Docker::connect_with_http_defaults().unwrap().version(),
        |version: Version| assert_eq!(version.os, "windows")
    )
}

#[cfg(feature = "test_tls")]
#[test]
fn test_version_tls() {
    rt_exec!(
        Docker::connect_with_tls_defaults().unwrap().version(),
        |version: Version| assert_eq!(version.os, "linux")
    )
}

#[cfg(unix)]
#[test]
fn test_downversioning() {
    let rt = Runtime::new().unwrap();

    env_logger::init();

    let docker = Docker::connect_with_unix(
        "unix:///var/run/docker.sock",
        120,
        &ClientVersion {
            major_version: 1,
            minor_version: 24,
        },
    )
    .unwrap();

    let fut = async move {
        let docker = &docker.negotiate_version().await.unwrap();

        &docker.version().await.unwrap();

        assert_eq!(
            format!("{}", &docker.client_version()),
            format!("{}", "1.24")
        );
    };
    rt.block_on(fut);

    rt.shutdown_now();
}
