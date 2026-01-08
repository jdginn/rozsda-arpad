mod osc;
mod shared;
mod traits;

use std::net::{SocketAddrV4, UdpSocket};
use std::str::FromStr;
use std::sync::Arc;

use clap::Parser;
use crossbeam_channel::bounded;
use rosc::OscMessage;

use osc::generated_osc::{Reaper, context_kind, dispatch_osc};
use osc::route_context::{ContextGateBuilder, OscGatedRouterBuilder};

use arpad_rust::track::track::{
    DataPayload, Direction, SendIndex, SendLevel, SendPan, TrackDataMsg, TrackManager, TrackMsg,
};

use crate::shared::Shared;
use crate::traits::Bind;

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

    let (a_send, a_rec) = bounded(128); // buffer size as needed
    let (b, _) = bounded(128); // buffer size as needed
    let (c, _) = bounded(128); // buffer size as needed
    TrackManager::start(a_rec.clone(), b.clone(), c.clone());

    let dispatcher = {
        let reaper = reaper.clone();
        move |msg: OscMessage| {
            reaper.with_mut(|reaper| {
                dispatch_osc(reaper, msg, |_| println!("Unhandled message"));
            })
        }
    };

    let mut router = OscGatedRouterBuilder::new(dispatcher)
        .add_layer({
            let reaper = reaper.clone();
            let a_send = a_send.clone();
            Box::new(
                ContextGateBuilder::<context_kind::Track>::new()
                    .add_key_route("/track/{guid}/index")
                    .with_initialization_callback(move |ctx, key_messages| {
                        println!(
                            "Initialized track context: {:?} with messages: {:?}",
                            ctx, key_messages
                        );
                        reaper.with_mut(|reaper| {
                            let track_guid = ctx.track_guid;
                            // Track Index
                            //
                            // For now, we aren't doing anything with this
                            reaper.track(track_guid.clone()).index().bind({
                                let track_guid = track_guid.clone();
                                let a_send = a_send.clone();
                                move |index| {
                                    a_send
                                        .try_send(TrackMsg::TrackDataMsg(TrackDataMsg {
                                            guid: track_guid.clone(),
                                            direction: Direction::Downstream,
                                            data: DataPayload::ReaperTrackIndex(Some(index.index)),
                                        }))
                                        .unwrap();
                                    println!(
                                        "Track {} index initial value: {:?}",
                                        track_guid.clone(),
                                        index
                                    )
                                }
                            });
                            // Track Name
                            reaper.track(track_guid.clone()).name().bind({
                                let track_guid = track_guid.clone();
                                let a_send = a_send.clone();
                                move |name| {
                                    a_send
                                        .try_send(TrackMsg::TrackDataMsg(TrackDataMsg {
                                            guid: track_guid.clone(),
                                            direction: Direction::Downstream,
                                            data: DataPayload::Name(name.name.clone()),
                                        }))
                                        .unwrap();
                                    println!(
                                        "Track {} name initial value: {:?}",
                                        track_guid.clone(),
                                        name
                                    )
                                }
                            });
                            // Track Selected
                            reaper.track(track_guid.clone()).selected().bind({
                                let track_guid = track_guid.clone();
                                let a_send = a_send.clone();
                                move |selected| {
                                    a_send
                                        .try_send(TrackMsg::TrackDataMsg(TrackDataMsg {
                                            guid: track_guid.clone(),
                                            direction: Direction::Downstream,
                                            data: DataPayload::Selected(selected.selected),
                                        }))
                                        .unwrap();
                                    println!(
                                        "Track {} selected initial value: {:?}",
                                        track_guid.clone(),
                                        selected
                                    )
                                }
                            });
                            // Track Muted
                            reaper.track(track_guid.clone()).mute().bind({
                                let track_guid = track_guid.clone();
                                let a_send = a_send.clone();
                                move |muted| {
                                    a_send
                                        .try_send(TrackMsg::TrackDataMsg(TrackDataMsg {
                                            guid: track_guid.clone(),
                                            direction: Direction::Downstream,
                                            data: DataPayload::Muted(muted.mute),
                                        }))
                                        .unwrap();
                                    println!(
                                        "Track {} muted initial value: {:?}",
                                        track_guid.clone(),
                                        muted
                                    )
                                }
                            });
                            // Track Soloed
                            reaper.track(track_guid.clone()).solo().bind({
                                let track_guid = track_guid.clone();
                                let a_send = a_send.clone();
                                move |soloed| {
                                    a_send
                                        .try_send(TrackMsg::TrackDataMsg(TrackDataMsg {
                                            guid: track_guid.clone(),
                                            direction: Direction::Downstream,
                                            data: DataPayload::Soloed(soloed.solo),
                                        }))
                                        .unwrap();
                                    println!(
                                        "Track {} soloed initial value: {:?}",
                                        track_guid.clone(),
                                        soloed
                                    )
                                }
                            });
                            // Track Armed
                            reaper.track(track_guid.clone()).rec_arm().bind({
                                let track_guid = track_guid.clone();
                                let a_send = a_send.clone();
                                move |rec_arm| {
                                    a_send
                                        .try_send(TrackMsg::TrackDataMsg(TrackDataMsg {
                                            guid: track_guid.clone(),
                                            direction: Direction::Downstream,
                                            data: DataPayload::Armed(rec_arm.rec_arm),
                                        }))
                                        .unwrap();
                                    println!(
                                        "Track {} armed initial value: {:?}",
                                        track_guid.clone(),
                                        rec_arm
                                    )
                                }
                            });
                            // Track Volume
                            reaper.track(track_guid.clone()).volume().bind({
                                let track_guid = track_guid.clone();
                                let a_send = a_send.clone();
                                move |volume| {
                                    a_send
                                        .try_send(TrackMsg::TrackDataMsg(TrackDataMsg {
                                            guid: track_guid.clone(),
                                            direction: Direction::Downstream,
                                            data: DataPayload::Volume(volume.volume),
                                        }))
                                        .unwrap();
                                    println!(
                                        "Track {} volume initial value: {:?}",
                                        track_guid.clone(),
                                        volume
                                    )
                                }
                            });
                            // Track Pan
                            reaper.track(track_guid.clone()).pan().bind({
                                let track_guid = track_guid.clone();
                                let a_send = a_send.clone();
                                move |pan| {
                                    a_send
                                        .try_send(TrackMsg::TrackDataMsg(TrackDataMsg {
                                            guid: track_guid.clone(),
                                            direction: Direction::Downstream,
                                            data: DataPayload::Pan(pan.pan),
                                        }))
                                        .unwrap();
                                    println!(
                                        "Track {} pan initial value: {:?}",
                                        track_guid.clone(),
                                        pan
                                    )
                                }
                            });
                        });
                    }),
            )
        })
        .add_layer({
            let reaper = reaper.clone();
            let a_send = a_send.clone();
            Box::new(
                ContextGateBuilder::<context_kind::TrackSend>::new()
                    .add_key_route("/track/{guid}/send/{send_index}/guid")
                    .with_initialization_callback(move |ctx, key_messages| {
                        let track_guid = ctx.track_guid.clone();
                        let send_index = ctx.send_index;
                        println!(
                            "Initialized track send context: {:?} with messages: {:?}",
                            ctx, key_messages
                        );
                        reaper.with_mut(|reaper| {
                            // Track Send GUID
                            reaper
                                .track(track_guid.clone())
                                .send(send_index)
                                .guid()
                                .bind({
                                    let track_guid = track_guid.clone();
                                    let a_send = a_send.clone();
                                    move |send_guid| {
                                        a_send
                                            .try_send(TrackMsg::TrackDataMsg(TrackDataMsg {
                                                guid: track_guid.clone(),
                                                direction: Direction::Downstream,
                                                data: DataPayload::SendIndex(SendIndex {
                                                    guid: send_guid.guid.clone(),
                                                    send_index,
                                                }),
                                            }))
                                            .unwrap();
                                        println!(
                                            "Track {} send {} guid initial value: {:?}",
                                            track_guid.clone(),
                                            send_index,
                                            send_guid
                                        )
                                    }
                                });
                            // Track Send Volume
                            reaper
                                .track(track_guid.clone())
                                .send(send_index)
                                .volume()
                                .bind({
                                    let track_guid = track_guid.clone();
                                    let a_send = a_send.clone();
                                    move |send_volume| {
                                        a_send
                                            .try_send(TrackMsg::TrackDataMsg(TrackDataMsg {
                                                guid: track_guid.clone(),
                                                direction: Direction::Downstream,
                                                data: DataPayload::SendLevel(SendLevel {
                                                    send_index,
                                                    level: send_volume.volume,
                                                }),
                                            }))
                                            .unwrap();
                                        println!(
                                            "Track {} send {} volume initial value: {:?}",
                                            track_guid.clone(),
                                            send_index,
                                            send_volume
                                        )
                                    }
                                });
                            // Track Send Pan
                            reaper
                                .track(track_guid.clone())
                                .send(send_index)
                                .pan()
                                .bind({
                                    let track_guid = track_guid.clone();
                                    let a_send = a_send.clone();
                                    move |send_pan| {
                                        a_send
                                            .try_send(TrackMsg::TrackDataMsg(TrackDataMsg {
                                                guid: track_guid.clone(),
                                                direction: Direction::Downstream,
                                                data: DataPayload::SendPan(SendPan {
                                                    send_index,
                                                    pan: send_pan.pan,
                                                }),
                                            }))
                                            .unwrap();
                                        println!(
                                            "Track {} send {} pan initial value: {:?}",
                                            track_guid.clone(),
                                            send_index,
                                            send_pan
                                        )
                                    }
                                });
                        });
                    }),
            )
        })
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
