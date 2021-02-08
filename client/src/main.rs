// SPDX-License-Identifier: Apache-2.0

#![deny(clippy::all)]
extern crate reqwest;

use ciborium::de::from_reader;
use config::*;
use franca::*;
use serde::*;
use std::path::PathBuf;
use structopt::StructOpt;


#[derive(StructOpt)]
pub struct Deploy {
    payload: PathBuf,
    keepmgr_addr: String,
    keepmgr_port: u16,
}

#[derive(StructOpt)]
//#[structopt(version=VERSION, author=AUTHORS.split(";").nth(0).unwrap())]
enum Options {
    //Info(Info),
    Deploy(Deploy),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct KeepMgr {
    //pub ipaddr: IpAddr,
    pub address: String,
    pub port: u16,
}

fn main() {
    let mut settings = config::Config::default();
    settings
        .merge(config::File::with_name("Client_config"))
        .unwrap()
        .merge(config::Environment::with_prefix("client"))
        .unwrap();
    
        match Options::from_args() {
            Options::Deploy(e) => deploy(e, &mut settings),
        }
}


pub fn deploy(deploy: Deploy, settings: &mut Config) {
    //TODO - implement
    let keepmgr = KeepMgr {
        address: deploy.keepmgr_addr,
        port: deploy.keepmgr_port,
    };
    let _try_uw = settings.set("user_workload", deploy.payload.to_str());
    let contracts: Vec<Contract> = list_contracts(&keepmgr).unwrap();
    if contracts.is_empty() {
        panic!("No contracts available");
    } else {
        for i in 0..contracts.len() {
            println!("Contract available for {:?}, uuid = {}", contracts[i].backend, contracts[i].uuid);
        }
    }
}

pub fn list_contracts(keepmgr: &KeepMgr) -> Result<Vec<Contract>, String> {
    let keep_mgr_url = format!("http://{}:{}/contracts/", keepmgr.address, keepmgr.port);

    println!("\nAbout to connect on {}", keep_mgr_url);

    let cbor_response: reqwest::blocking::Response = reqwest::blocking::Client::builder()
        .build()
        .unwrap()
        .get(&keep_mgr_url)
        //.body()
        .send()
        .expect("Problem getting contracts");

    let cbytes: &[u8] = &cbor_response.bytes().unwrap();
    //println!("cbytes len = {}", cbytes.len());
    let crespbytes = cbytes.as_ref();
    let contractvec: Vec<Contract> = from_reader(&crespbytes[..]).unwrap();

    Ok(contractvec)
}