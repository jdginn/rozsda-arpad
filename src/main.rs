mod osc;
mod shared;
mod traits;

use std::net::{SocketAddrV4, UdpSocket};
use std::str::FromStr;
use std::sync::{Arc, Mutex};

use clap::Parser;
use rosc::OscMessage;

use osc::generated_osc::{Reaper, context_kind, dispatch_osc};
use osc::route_context::{ContextGateBuilder, OscGatedRouterBuilder};

use crate::shared::Shared;
use crate::traits::{Bind, Query, Set};

#[derive(Parser)]
struct Cli {
    #[clap(short, long, default_value = "0.0.0.0:9000")]
    osc_address: String,
}

fn main() {
    let cli = Cli::parse();
    let socket_addr = SocketAddrV4::from_str(&cli.osc_address)
        .unwrap_or_else(|_| panic!("couldn't parse address {:?}", cli.osc_address));
    let socket = UdpSocket::bind(socket_addr)
        .unwrap_or_else(|_| panic!("couldn't bind to address {:?}", cli.osc_address));

    let reaper = Shared::new(Reaper::new(Arc::new(socket.try_clone().unwrap())));

    let dispatcher = {
        let reaper = reaper.clone();
        move |msg: OscMessage| {
            reaper.with_mut(|reaper| {
                dispatch_osc(reaper, msg, |_| println!("Unhandled message"));
            })
        }
    };

    let mut router = OscGatedRouterBuilder::new(dispatcher)
        .add_layer(Box::new(
            ContextGateBuilder::<context_kind::Track>::new()
                .add_key_route("/track/{guid}/index")
                .with_initialization_callback(move |ctx, key_messages| {
                    println!(
                        "Initialized track context: {:?} with messages: {:?}",
                        ctx, key_messages
                    );
                    reaper.with_mut(|reaper| {
                        let track_guid = ctx.track_guid;
                        // Track Selected
                        reaper.track(track_guid.clone()).selected().bind({
                            let track_guid = track_guid.clone();
                            move |selected| {
                                println!(
                                    "Track {} selected initial value: {:?}",
                                    track_guid.clone(),
                                    selected
                                )
                            }
                        });
                        // Track Index
                        reaper.track(track_guid.clone()).index().bind({
                            let track_guid = track_guid.clone();
                            move |index| {
                                println!(
                                    "Track {} index initial value: {:?}",
                                    track_guid.clone(),
                                    index
                                )
                            }
                        });
                    });
                }),
        ))
        .build()
        .unwrap();

    println!("Listening on {}", cli.osc_address);
    let mut buf = [0u8; rosc::decoder::MTU];
    loop {
        match socket.recv_from(&mut buf) {
            Ok((size, addr)) => {
                println!("Received packet with size {} from: {}", size, addr);
                let (_, packet) = rosc::decoder::decode_udp(&buf[..size]).unwrap();
                router.dispatch_osc(packet);
                // handle_packet(packet);
            }
            Err(e) => {
                println!("Error receiving from socket: {}", e);
                break;
            }
        }
    }
}
