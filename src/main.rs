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
    DataPayload, Direction, FXEnabled, FXGuid, FXName, FXParamMax, FXParamMin, FXParamName,
    FXParamValue, SendIndex, SendLevel, SendPan, TrackDataMsg, TrackManager, TrackMsg,
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
                            reaper.track_index(track_guid.clone()).bind({
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
                            reaper.track_name(track_guid.clone()).bind({
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
                            reaper.track_selected(track_guid.clone()).bind({
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
                            reaper.track_mute(track_guid.clone()).bind({
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
                            reaper.track_solo(track_guid.clone()).bind({
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
                            reaper.track_rec_arm(track_guid.clone()).bind({
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
                            reaper.track_volume(track_guid.clone()).bind({
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
                            reaper.track_pan(track_guid.clone()).bind({
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
                                .track_send_guid(track_guid.clone(), send_index)
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
                                .track_send_volume(track_guid.clone(), send_index)
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
                            reaper.track_send_pan(track_guid.clone(), send_index).bind({
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
        .add_layer({
            let reaper = reaper.clone();
            let a_send = a_send.clone();
            Box::new(
                ContextGateBuilder::<context_kind::TrackFx>::new()
                    .add_key_route("/track/{guid}/fx/{fx_idx}/guid")
                    .with_initialization_callback(move |ctx, key_messages| {
                        let track_guid = ctx.track_guid.clone();
                        let a_send = a_send.clone();
                        println!(
                            "Initialized track fxcontext: {:?} with messages: {:?}",
                            ctx, key_messages
                        );
                        reaper.with_mut(|reaper| {
                            // Track FX guid
                            reaper.track_fx_guid(track_guid.clone(), ctx.fx_idx).bind({
                                let track_guid = track_guid.clone();
                                let a_send = a_send.clone();
                                move |fx_guid| {
                                    a_send
                                        .try_send(TrackMsg::TrackDataMsg(TrackDataMsg {
                                            guid: track_guid.clone(),
                                            direction: Direction::Downstream,
                                            data: DataPayload::FXGuid(FXGuid {
                                                fx_index: ctx.fx_idx,
                                                guid: fx_guid.guid.clone(),
                                            }),
                                        }))
                                        .unwrap();
                                }
                            });
                            // Track FX Name
                            reaper.track_fx_name(track_guid.clone(), ctx.fx_idx).bind({
                                let track_guid = track_guid.clone();
                                let a_send = a_send.clone();
                                move |fx_name| {
                                    a_send
                                        .try_send(TrackMsg::TrackDataMsg(TrackDataMsg {
                                            guid: track_guid.clone(),
                                            direction: Direction::Downstream,
                                            data: DataPayload::FXName(FXName {
                                                fx_index: ctx.fx_idx,
                                                name: fx_name.name.clone(),
                                            }),
                                        }))
                                        .unwrap();
                                    println!(
                                        "Track {} fx {} name initial value: {:?}",
                                        track_guid.clone(),
                                        ctx.fx_idx,
                                        fx_name
                                    )
                                }
                            });
                            // Track FX Enabled
                            reaper
                                .track_fx_enabled(track_guid.clone(), ctx.fx_idx)
                                .bind({
                                    let track_guid = track_guid.clone();
                                    let a_send = a_send.clone();
                                    move |fx_enabled| {
                                        a_send
                                            .try_send(TrackMsg::TrackDataMsg(TrackDataMsg {
                                                guid: track_guid.clone(),
                                                direction: Direction::Downstream,
                                                data: DataPayload::FXEnabled(FXEnabled {
                                                    fx_index: ctx.fx_idx,
                                                    enabled: fx_enabled.enabled,
                                                }),
                                            }))
                                            .unwrap();
                                        println!(
                                            "Track {} fx {} enabled initial value: {:?}",
                                            track_guid.clone(),
                                            ctx.fx_idx,
                                            fx_enabled
                                        )
                                    }
                                });
                        })
                    }),
            )
        })
        .add_layer({
            let reaper = reaper.clone();
            let a_send = a_send.clone();
            Box::new(
                ContextGateBuilder::<context_kind::TrackFxParam>::new()
                    .add_key_route("/track/{guid}/fx/{fx_idx}/param/{param_idx}/name")
                    .with_initialization_callback(move |ctx, key_messages| {
                        let track_guid = ctx.track_guid.clone();
                        let a_send = a_send.clone();
                        println!(
                            "Initialized track fx param context: {:?} with messages: {:?}",
                            ctx, key_messages
                        );
                        reaper.with_mut(|reaper| {
                            // Track FX Param Name
                            reaper
                                .track_fx_param_name(track_guid.clone(), ctx.fx_idx, ctx.param_idx)
                                .bind({
                                    let track_guid = track_guid.clone();
                                    let a_send = a_send.clone();
                                    move |fx_param_name| {
                                        a_send
                                            .try_send(TrackMsg::TrackDataMsg(TrackDataMsg {
                                                guid: track_guid.clone(),
                                                direction: Direction::Downstream,
                                                data: DataPayload::FXParamName(FXParamName {
                                                    fx_index: ctx.fx_idx,
                                                    param_index: ctx.param_idx,
                                                    name: fx_param_name.param_name.clone(),
                                                }),
                                            }))
                                            .unwrap();
                                        println!(
                                            "Track {} fx {} param {} name initial value: {:?}",
                                            track_guid.clone(),
                                            ctx.fx_idx,
                                            ctx.param_idx,
                                            fx_param_name
                                        )
                                    }
                                });
                            // Track FX Param Value
                            reaper
                                .track_fx_param_value(track_guid.clone(), ctx.fx_idx, ctx.param_idx)
                                .bind({
                                    let track_guid = track_guid.clone();
                                    let a_send = a_send.clone();
                                    move |fx_param_value| {
                                        a_send
                                            .try_send(TrackMsg::TrackDataMsg(TrackDataMsg {
                                                guid: track_guid.clone(),
                                                direction: Direction::Downstream,
                                                data: DataPayload::FXParamValue(FXParamValue {
                                                    fx_index: ctx.fx_idx,
                                                    param_index: ctx.param_idx,
                                                    value: fx_param_value.value,
                                                }),
                                            }))
                                            .unwrap();
                                        println!(
                                            "Track {} fx {} param {} value initial value: {:?}",
                                            track_guid.clone(),
                                            ctx.fx_idx,
                                            ctx.param_idx,
                                            fx_param_value
                                        )
                                    }
                                });
                            // Track FX Param Min
                            reaper
                                .track_fx_param_min(track_guid.clone(), ctx.fx_idx, ctx.param_idx)
                                .bind({
                                    let track_guid = track_guid.clone();
                                    let a_send = a_send.clone();
                                    move |fx_param_min| {
                                        a_send
                                            .try_send(TrackMsg::TrackDataMsg(TrackDataMsg {
                                                guid: track_guid.clone(),
                                                direction: Direction::Downstream,
                                                data: DataPayload::FXParamMin(FXParamMin {
                                                    fx_index: ctx.fx_idx,
                                                    param_index: ctx.param_idx,
                                                    min: fx_param_min.min,
                                                }),
                                            }))
                                            .unwrap();
                                        println!(
                                            "Track {} fx {} param {} min initial value: {:?}",
                                            track_guid.clone(),
                                            ctx.fx_idx,
                                            ctx.param_idx,
                                            fx_param_min
                                        )
                                    }
                                });
                            // Track FX Param Max
                            reaper
                                .track_fx_param_max(track_guid.clone(), ctx.fx_idx, ctx.param_idx)
                                .bind({
                                    let track_guid = track_guid.clone();
                                    let a_send = a_send.clone();
                                    move |fx_param_max| {
                                        a_send
                                            .try_send(TrackMsg::TrackDataMsg(TrackDataMsg {
                                                guid: track_guid.clone(),
                                                direction: Direction::Downstream,
                                                data: DataPayload::FXParamMax(FXParamMax {
                                                    fx_index: ctx.fx_idx,
                                                    param_index: ctx.param_idx,
                                                    max: fx_param_max.max,
                                                }),
                                            }))
                                            .unwrap();
                                        println!(
                                            "Track {} fx {} param {} max initial value: {:?}",
                                            track_guid.clone(),
                                            ctx.fx_idx,
                                            ctx.param_idx,
                                            fx_param_max
                                        )
                                    }
                                });
                        })
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
