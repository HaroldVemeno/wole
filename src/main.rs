use std::{collections::HashMap, env, io};

use pnet::{datalink::{self, NetworkInterface}, util::MacAddr};
use serde::{Serialize, Deserialize};
use easy_config_store::ConfigStore;
use clap::Parser;

mod raw;
mod udp;

#[derive(Default, PartialEq, Debug, Clone, Serialize, Deserialize)]
struct Config {
    aliases: HashMap<String, String>,
}

#[derive(Parser, Debug, Clone)]
#[command(version, about, long_about = None)]
struct Args {

    // #[arg(short='4', long)]
    // ipv4: bool,

    // #[arg(short='6', long)]
    // ipv6: bool,

    #[arg(short, long)]
    save: Option<String>,

    #[arg(short, long)]
    raw: bool,

    #[arg(short, long)]
    if_name: Option<String>,

    destination: String,
}

fn get_config() -> io::Result<ConfigStore<Config>> {
    let mut conf = if let Ok(conf_dir) = env::var("XDG_CONFIG_HOME") {
        conf_dir.into()
    } else if let Some(mut home) = env::home_dir() {
        home.push(".config");
        home
    } else {
        panic!("No home directory set!")
    };

    conf.push("wole.toml");

     Ok(ConfigStore::<Config>::read(conf, None).expect("Couldn't read config"))
}

fn main() -> io::Result<()> {
    let mut config = get_config()?;
    let args = Args::parse();
    let interfaces: Vec<NetworkInterface> = {
        let ifs = datalink::interfaces().into_iter();
        if let Some(name) = args.if_name {
            ifs.filter(|ifc| ifc.name == name).collect()
        } else {
            ifs.filter(|ifc| !ifc.is_loopback() && ifc.is_up() && ifc.mac.is_some()).collect()
        }
    };
    let mac: MacAddr = if let Ok(mac) = args.destination.parse::<MacAddr>() {
        mac
    } else if let Some(mac_str) = config.aliases.get(&args.destination) {
        mac_str.parse::<MacAddr>().expect("Invalid mac address in config")
    } else {
        panic!("Unable to interpret destination");
    };
    if args.raw {
        for interface in &interfaces {
            println!("Trying interface {}", interface.name);
            raw::send_with_interface(mac, interface)?;
        }
    } else {
        udp::send(mac)?;
    }
    if let Some(alias) = args.save {
        config.aliases.insert(alias, mac.to_string());
        config.save().expect("Couldn't write config");
    }

    Ok(())
}
