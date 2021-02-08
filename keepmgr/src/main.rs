// SPDX-License-Identifier: Apache-2.0

#![deny(clippy::all)]

use koine::{Backend, Contract};

use config::*;
use serde::Serialize;
use structopt::StructOpt;
use tokio::net::{TcpListener};
use tokio_stream::wrappers::TcpListenerStream;
use uuid::Uuid;
use warp::http::header::CONTENT_TYPE;
use warp::http::{Response, StatusCode};
use warp::Filter;

const CONTRACTS: &[Contract] = &[
    Contract {
        uuid: Uuid::from_u128(0xe6234733_513a_4883_981a_bfa972fa706b),
        backend: Backend::Nil,
    },
    Contract {
        uuid: Uuid::from_u128(0x0afa438e_acaa_4158_9518_ad59256def34),
        backend: Backend::Kvm,
    },
    Contract {
        uuid: Uuid::from_u128(0x31a41b53_cb9e_447b_bfa2_bfb8e6e42ff9),
        backend: Backend::Sev,
    },
    Contract {
        uuid: Uuid::from_u128(0xea392851_3435_42d3_a4ad_c4e5e5c6c4c6),
        backend: Backend::Sgx,
    },
];

trait ContractExt {
    fn is_supported(&self) -> bool;
}

impl ContractExt for Contract {
    fn is_supported(&self) -> bool {
        use std::path::Path;

        match self.backend {
            Backend::Nil => true,
            Backend::Kvm => Path::new("/dev/kvm").exists(),
            Backend::Sev => Path::new("/dev/sev").exists(),
            Backend::Sgx => Path::new("/dev/sgx_enclave").exists(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ContractMgr {
    pub address: String,
    pub port: u16,
}

#[derive(Debug)]
enum Listener {
    Unix(std::os::unix::net::UnixListener),
    Tcp(std::net::TcpListener),
}

impl std::str::FromStr for Listener {
    type Err = std::io::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use nix::sys::socket::{getsockname, SockAddr};
        use std::io::ErrorKind;
        use std::net::TcpListener as Tcp;
        use std::os::unix::io::{FromRawFd, RawFd};
        use std::os::unix::net::UnixListener as Unix;

        if let Ok(fd) = RawFd::from_str(s) {
            return match getsockname(fd).map_err(|_| ErrorKind::InvalidInput)? {
                SockAddr::Unix(..) => Ok(Listener::Unix(unsafe { Unix::from_raw_fd(fd) })),
                SockAddr::Inet(..) => Ok(Listener::Tcp(unsafe { Tcp::from_raw_fd(fd) })),
                _ => Err(ErrorKind::InvalidInput.into()),
            };
        }

        Ok(match s.chars().next() {
            Some('/') => Listener::Unix(Unix::bind(s)?),
            _ => Listener::Tcp(Tcp::bind(s)?),
        })
    }
}


#[derive(StructOpt)]
struct Run {}

#[derive(Debug, StructOpt)]
#[structopt(name = "keepmgr", about = "Manages keeps.")]
struct Options {
    /// The listening socket address or fd
    listen: Listener,
}

fn cborize<T: Serialize>(item: &T) -> Vec<u8> {
    let mut buffer = Vec::new();
    ciborium::ser::into_writer(&item, &mut buffer).unwrap();
    buffer
}

fn error(code: StatusCode) -> Response<Vec<u8>> {
    Response::builder().status(code).body(Vec::new()).unwrap()
}

async fn serve<I>(incoming: I) -> tokio::io::Result<()>
where
    I: futures_core::stream::TryStream + Send,
    I::Ok: tokio::io::AsyncRead + tokio::io::AsyncWrite + Send + 'static + Unpin,
    I::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    // Client is requesting details of all contracts.

    let get_contracts = warp::path!("contracts")
        .and(warp::filters::method::get())
        //.and(with_contactmgr(contractmgr))
        .and_then(list_contracts);
    
    // Client is requesting details of a single contract.
    let get_contracts_uuid = warp::path!("contracts" / Uuid)
        .and(warp::filters::method::get())
        .map(|cuuid| {
            // TODO: fetch contracts from the contractmgr
            let contracts: Vec<Contract> = CONTRACTS
                .iter()
                .cloned()
                .filter(Contract::is_supported)
                .collect();

            match contracts.iter().find(|c| c.uuid == cuuid) {
                None => error(StatusCode::NOT_FOUND),
                Some(contract) => Response::builder()
                    .status(StatusCode::OK)
                    .header(CONTENT_TYPE, "application/cbor")
                    .body(cborize(&contract))
                    .unwrap(),
            }
        });

    let routes = get_contracts.or(get_contracts_uuid);
    warp::serve(routes).run_incoming(incoming).await;
    Ok(())
}

#[tokio::main]
async fn main() -> tokio::io::Result<()> {
    let mut settings = config::Config::default();
    settings
        .merge(File::with_name("Keepmgr_config"))
        .unwrap()
        .merge(Environment::with_prefix("keepmgr"))
        .unwrap();

    let listen_type: String = settings.get("keepmgr_type").unwrap();
    match listen_type.as_ref() {
        "tcp" => {
            let my_address: String = settings.get("keepmgr_address").unwrap();
            let my_port: u16 = settings.get("keepmgr_port").unwrap();
            println!("Address = {}, port = {}", &my_address, &my_port);
            let full_address = format!("{}:{}", my_address, my_port);
            println!("Binding to {}", full_address);
            let stdlisten = std::net::TcpListener::bind(full_address).unwrap();
            let listen = TcpListener::from_std(stdlisten).unwrap();
            let stream = TcpListenerStream::new(listen);
            serve(stream).await
        }
        _ => {
            panic!("Unimplemented");
        }
    }

}

pub async fn list_contracts(
 //       available_contracts: ContractList,
    ) -> Result<impl warp::Reply, warp::Rejection> {
    println!("Calling list_contracts");
    let conl: Vec<Contract> = CONTRACTS.to_vec();

    Ok(cborize(&conl))
}
