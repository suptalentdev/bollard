extern crate docker;

use docker::Docker;
use docker::errors::*;
use std::io::{self, Write};

fn find_all_exported_ports() -> Result<()> {
    let docker = try!(Docker::connect_with_defaults());
    let containers = try!(docker.get_containers(false));
    for container in &containers {
        let info = try!(docker.get_container_info(&container));

        // TODO: Actually find the listening port numbers here.
        println!("{:#?}", &info);
    }
    Ok(())
}

fn main() {
    if let Err(err) = find_all_exported_ports() {
        write!(io::stderr(), "Error: ").unwrap();
        for e in err.iter() {
            write!(io::stderr(), "{}\n", e).unwrap();
        }
    }
}
