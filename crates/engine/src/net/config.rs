use bevy::prelude::*;
use lightyear::prelude::*;
use lightyear::server::config::{NetcodeConfig, ServerConfig};
use rand::seq::SliceRandom;
use rand::RngCore;
use std::net::*;
use std::time::Duration;

use crate::net::protocol::ProtocolPlugin;

use super::NetworkType;

pub const REPLICATION_INTERVAL: Duration = Duration::from_millis(100);

pub fn setup(
    app: &mut App,
    network_type: NetworkType,
    server_port: Option<u16>,
    client_addr: Option<String>,
) {
    info!(
        "entering network setup with type {:?}, server_port {:?}, client_addr {:?}",
        network_type, server_port, client_addr
    );
    app.world_mut()
        .get_resource_mut::<NextState<NetworkType>>()
        .unwrap()
        .set(network_type);
    match network_type {
        NetworkType::Singleplayer | NetworkType::Inactive => {
            info!("skipping network setup");
            return;
        }
        _ => (),
    };

    let replication = ReplicationConfig {
        send_interval: REPLICATION_INTERVAL,
        ..default()
    };
    let shared = SharedConfig {
        server_replication_send_interval: REPLICATION_INTERVAL,
        tick: TickConfig::new(Duration::from_secs_f64(1. / crate::physics::TPS)),
        mode: network_type.to_network_mode(),
        ..default()
    };

    if matches!(network_type, NetworkType::Server | NetworkType::Host) {
        let server_net_configs = vec![server::NetConfig::Netcode {
            config: NetcodeConfig::default().with_client_timeout_secs(10),
            io: server::IoConfig {
                transport: server::ServerTransport::UdpSocket(SocketAddr::new(
                    Ipv4Addr::UNSPECIFIED.into(),
                    server_port.unwrap_or(15155),
                )),
                ..default()
            },
        }];
        let server = ServerConfig {
            shared,
            net: server_net_configs,
            replication,
            ..default()
        };
        app.add_plugins((
            lightyear::prelude::server::ServerPlugins { config: server },
            super::server::ServerPlugin { network_type },
        ));
        info!("added server plugins!");
    }

    if matches!(network_type, NetworkType::Client | NetworkType::Host) {
        //pick a random ip address from the dns resolution
        let client_socket = client_addr
            .and_then(|addr| addr.to_socket_addrs().ok())
            .and_then(|iter| {
                iter.collect::<Vec<_>>()
                    .choose(&mut rand::thread_rng())
                    .copied()
            });
        let client_net_config = if matches!(network_type.to_network_mode(), Mode::HostServer) {
            client::NetConfig::Local { id: 0 }
        } else {
            // os should assign a random port
            const CLIENT_ADDR: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0);
            info!("resolved server ip address: {:?}", client_socket.unwrap());
            client::NetConfig::Netcode {
                auth: client::Authentication::Manual {
                    server_addr: client_socket.unwrap(),
                    client_id: rand::thread_rng().next_u64(),
                    private_key: Key::default(),
                    protocol_id: 0,
                },
                config: client::NetcodeConfig::default(),
                io: client::IoConfig {
                    transport: client::ClientTransport::UdpSocket(CLIENT_ADDR),
                    ..default()
                },
            }
        };
        let client = client::ClientConfig {
            shared,
            net: client_net_config,
            ..default()
        };
        app.add_plugins((
            lightyear::prelude::client::ClientPlugins { config: client },
            super::client::ClientPlugin { network_type },
        ));
        info!("added client plugins!");
    }
    app.add_plugins(ProtocolPlugin);
    info!("done setting up network");
}
