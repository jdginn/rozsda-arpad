mod osc;
mod shared;
mod traits;

use std::net::{SocketAddrV4, UdpSocket};
use std::str::FromStr;
use std::sync::Arc;

use clap::Parser;
use crossbeam_channel::{bounded, select};
use midir::{Ignore, MidiInput, MidiInputPort, MidiOutput, MidiOutputConnection};
use rosc::OscMessage;

use osc::generated_osc;
use osc::generated_osc::{Reaper, context_kind, dispatch_osc};
use osc::route_context::{ContextGateBuilder, OscGatedRouterBuilder};

use arpad_rust::midi::xtouch;
use arpad_rust::modes::mode_manager;
use arpad_rust::track::track;

use crate::osc::generated_osc::TrackMuteArgs;
use crate::shared::Shared;
use crate::traits::{Bind, Set};

#[derive(Parser)]
struct Cli {
    #[clap(short, long, default_value = "0.0.0.0:9091")]
    osc_address: String,
    #[clap(short, long, default_value = "0.0.0.0:9090")]
    dev_osc_address: String,
}

fn find_xtouch_ports() -> Result<(MidiInputPort, MidiOutputConnection), Box<dyn std::error::Error>>
{
    let mut midi_in = MidiInput::new("midir input port sniff")?;
    midi_in.ignore(Ignore::None);
    let midi_out = MidiOutput::new("midir output port sniff")?;

    let mut input_port = None;
    let mut output_port = None;

    for (i, p) in midi_in.ports().iter().enumerate() {
        if input_port.is_none() && midi_in.port_name(p)? == "X-Touch INT" {
            input_port = Some(p.clone());
            break;
        }
    }
    for (i, p) in midi_out.ports().iter().enumerate() {
        if output_port.is_none() && midi_out.port_name(p)? == "X-Touch INT" {
            output_port = Some(p.clone());
            break;
        }
    }

    if input_port.is_none() {
        return Err("Could not find X-Touch MIDI input port".into());
    }

    println!("Found input port {}", input_port.clone().unwrap().id());

    if let Some(output_port) = output_port {
        println!(
            "Connecting to output port '{}' ...",
            midi_out.port_name(&output_port)?
        );
        let output_connection = midi_out.connect(&output_port, "midir-test")?;
        Ok((input_port.unwrap(), output_connection))
    } else {
        Err("Could not find X-Touch MIDI output port".into())
    }
}

fn main() {
    let cli = Cli::parse();
    let socket_addr = SocketAddrV4::from_str(&cli.osc_address)
        .unwrap_or_else(|_| panic!("couldn't parse address {:?}", cli.osc_address));
    let dev_addr = SocketAddrV4::from_str(&cli.dev_osc_address)
        .unwrap_or_else(|_| panic!("couldn't deviceaddress {:?}", cli.dev_osc_address));
    let socket = UdpSocket::bind(socket_addr)
        .unwrap_or_else(|_| panic!("couldn't bind to address {:?}", cli.osc_address));
    socket.connect(dev_addr).unwrap_or_else(|_| {
        panic!(
            "couldn't connect to device address {:?}",
            cli.dev_osc_address
        )
    });

    let reaper = Shared::new(Reaper::new(Arc::new(socket.try_clone().unwrap())));

    let (from_reaper_tx, from_reaper_rx) = bounded(128); // buffer size as needed
    let (to_reaper_tx, to_reaper_rx) = bounded(128); // buffer size as needed
    let (from_track_manager_tx, from_track_manager_rx) = bounded(128); // buffer size as needed
    let (to_track_manager_tx, to_track_manager_rx) = bounded(128); // buffer size as needed
    let (from_mode_manager_tx, from_mode_manager_rx) = bounded(128); // buffer size as needed
    let (to_mode_manager_tx, to_mode_manager_rx) = bounded(128); // buffer size as needed
    track::TrackManager::start(
        from_reaper_rx.clone(),
        to_reaper_tx.clone(),
        to_track_manager_rx.clone(),
        from_track_manager_tx.clone(),
    );
    mode_manager::ModeManager::start(
        from_track_manager_rx.clone(),
        to_track_manager_tx.clone(),
        to_mode_manager_rx.clone(),
        from_mode_manager_tx.clone(),
    );
    let (input_port, output_connection) = match find_xtouch_ports() {
        Ok(ports) => ports,
        Err(err) => {
            println!("Error finding XTouch MIDI ports: {}", err);
            println!("Please ensure XTouch is connected and try again.");
            return;
        }
    };
    xtouch::XTouchBuilder::new(input_port, output_connection, 8)
        .build(from_mode_manager_rx.clone(), to_mode_manager_tx.clone());

    let dispatcher = {
        let reaper = reaper.clone();
        move |msg: OscMessage| {
            println!("Received OSC message: {:?}", msg);
            reaper.with_mut(|reaper| {
                let msg_clone = msg.clone();
                dispatch_osc(
                    reaper,
                    msg,
                    |_| println!("Unhandled message: {:?}", msg_clone),
                    |msg, err| println!("Error dispatching message {:?}: {:?}", msg, err),
                );
            })
        }
    };

    let mut router = OscGatedRouterBuilder::new(dispatcher)
        .add_layer({
            let reaper = reaper.clone();
            let a_send = from_reaper_tx.clone();
            Box::new(
                ContextGateBuilder::<context_kind::Track>::new()
                    .add_key_route("/track/{guid}/index")
                    .with_initialization_callback(move |ctx, key_messages| {
                        reaper.with_mut(|reaper| {
                            println!(
                                "Binding track context for track guid: {:?} with messages: {:?}",
                                ctx.track_guid, key_messages
                            );
                            let track_guid = ctx.track_guid;
                            // Track Index
                            //
                            // For now, we aren't doing anything with this
                            reaper.track_index(track_guid).bind({
                                let a_send = a_send.clone();
                                move |index| {
                                    a_send
                                        .try_send(
                                            track::ReaperTrackIndex {
                                                track_guid,
                                                track_index: Some(index.index),
                                            }
                                            .into(),
                                        )
                                        .unwrap();
                                    println!(
                                        "Track {} index initial value: {:?}",
                                        track_guid.clone(),
                                        index
                                    )
                                }
                            });
                            // Track Name
                            reaper.track_name(track_guid).bind({
                                let a_send = a_send.clone();
                                move |name| {
                                    a_send
                                        .try_send(
                                            track::Name {
                                                track_guid,
                                                name: name.name.clone(),
                                            }
                                            .into(),
                                        )
                                        .unwrap();
                                    println!(
                                        "Track {} name initial value: {:?}",
                                        track_guid.clone(),
                                        name
                                    )
                                }
                            });
                            // Track Selected
                            reaper.track_selected(track_guid).bind({
                                let a_send = a_send.clone();
                                move |selected| {
                                    a_send
                                        .try_send(
                                            track::Selected {
                                                track_guid,
                                                selected: selected.selected,
                                            }
                                            .into(),
                                        )
                                        .unwrap();
                                    println!(
                                        "Track {} selected initial value: {:?}",
                                        track_guid.clone(),
                                        selected
                                    )
                                }
                            });
                            // Track Muted
                            reaper.track_mute(track_guid).bind({
                                let a_send = a_send.clone();
                                move |muted| {
                                    a_send
                                        .try_send(
                                            track::Muted {
                                                track_guid,
                                                muted: muted.mute,
                                            }
                                            .into(),
                                        )
                                        .unwrap();
                                    println!(
                                        "Track {} muted initial value: {:?}",
                                        track_guid.clone(),
                                        muted
                                    )
                                }
                            });
                            // Track Soloed
                            reaper.track_solo(track_guid).bind({
                                let a_send = a_send.clone();
                                move |soloed| {
                                    a_send
                                        .try_send(
                                            track::Soloed {
                                                track_guid,
                                                soloed: soloed.solo,
                                            }
                                            .into(),
                                        )
                                        .unwrap();
                                    println!(
                                        "Track {} soloed initial value: {:?}",
                                        track_guid.clone(),
                                        soloed
                                    )
                                }
                            });
                            // Track Armed
                            reaper.track_rec_arm(track_guid).bind({
                                let a_send = a_send.clone();
                                move |rec_arm| {
                                    a_send
                                        .try_send(
                                            track::Armed {
                                                track_guid,
                                                armed: rec_arm.rec_arm,
                                            }
                                            .into(),
                                        )
                                        .unwrap();
                                    println!(
                                        "Track {} armed initial value: {:?}",
                                        track_guid.clone(),
                                        rec_arm
                                    )
                                }
                            });
                            // Track Volume
                            reaper.track_volume(track_guid).bind({
                                let a_send = a_send.clone();
                                move |volume| {
                                    a_send
                                        .try_send(
                                            track::Volume {
                                                track_guid,
                                                volume: volume.volume,
                                            }
                                            .into(),
                                        )
                                        .unwrap();
                                    println!(
                                        "Track {} volume initial value: {:?}",
                                        track_guid.clone(),
                                        volume
                                    )
                                }
                            });
                            // Track Pan
                            reaper.track_pan(track_guid).bind({
                                let a_send = a_send.clone();
                                move |pan| {
                                    a_send
                                        .try_send(
                                            track::Pan {
                                                track_guid,
                                                pan: pan.pan,
                                            }
                                            .into(),
                                        )
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
            let a_send = from_reaper_tx.clone();
            Box::new(
                ContextGateBuilder::<context_kind::TrackSend>::new()
                    .add_key_route("/track/{guid}/send/{send_index}/guid")
                    .with_initialization_callback(move |ctx, key_messages| {
                        let track_guid = ctx.track_guid;
                        let send_index = ctx.send_index;
                        println!(
                            "Initialized track send context: {:?} with messages: {:?}",
                            ctx, key_messages
                        );
                        reaper.with_mut(|reaper| {
                            // Track Send GUID
                            reaper.track_send_guid(track_guid, send_index).bind({
                                let a_send = a_send.clone();
                                move |send_guid| {
                                    a_send
                                        .try_send(
                                            track::SendIndex {
                                                track_guid,
                                                send_index,
                                                send_guid: send_guid.guid,
                                            }
                                            .into(),
                                        )
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
                            reaper.track_send_volume(track_guid, send_index).bind({
                                let a_send = a_send.clone();
                                move |send_volume| {
                                    a_send
                                        .try_send(
                                            track::SendLevel {
                                                track_guid,
                                                send_index,
                                                level: send_volume.volume,
                                            }
                                            .into(),
                                        )
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
                            reaper.track_send_pan(track_guid, send_index).bind({
                                let a_send = a_send.clone();
                                move |send_pan| {
                                    a_send
                                        .try_send(
                                            track::SendPan {
                                                track_guid,
                                                send_index,
                                                pan: send_pan.pan,
                                            }
                                            .into(),
                                        )
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
            let a_send = from_reaper_tx.clone();
            Box::new(
                ContextGateBuilder::<context_kind::TrackFx>::new()
                    .add_key_route("/track/{guid}/fx/{fx_idx}/guid")
                    .with_initialization_callback(move |ctx, key_messages| {
                        let track_guid = ctx.track_guid;
                        let a_send = a_send.clone();
                        println!(
                            "Initialized track fxcontext: {:?} with messages: {:?}",
                            ctx, key_messages
                        );
                        reaper.with_mut(|reaper| {
                            // Track FX guid
                            reaper.track_fx_guid(track_guid, ctx.fx_idx).bind({
                                let a_send = a_send.clone();
                                move |fx_guid| {
                                    a_send
                                        .try_send(
                                            track::FXGuid {
                                                track_guid,
                                                fx_index: ctx.fx_idx,
                                                guid: fx_guid.guid,
                                            }
                                            .into(),
                                        )
                                        .unwrap();
                                }
                            });
                            // Track FX Name
                            reaper.track_fx_name(track_guid, ctx.fx_idx).bind({
                                let a_send = a_send.clone();
                                move |fx_name| {
                                    a_send
                                        .try_send(
                                            track::FXName {
                                                track_guid,
                                                fx_index: ctx.fx_idx,
                                                name: fx_name.name.clone(),
                                            }
                                            .into(),
                                        )
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
                            reaper.track_fx_enabled(track_guid, ctx.fx_idx).bind({
                                let a_send = a_send.clone();
                                move |fx_enabled| {
                                    a_send
                                        .try_send(
                                            track::FXEnabled {
                                                track_guid,
                                                fx_index: ctx.fx_idx,
                                                enabled: fx_enabled.enabled,
                                            }
                                            .into(),
                                        )
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
            let a_send = from_reaper_tx.clone();
            Box::new(
                ContextGateBuilder::<context_kind::TrackFxParam>::new()
                    .add_key_route("/track/{guid}/fx/{fx_idx}/param/{param_idx}/name")
                    .with_initialization_callback(move |ctx, key_messages| {
                        let track_guid = ctx.track_guid;
                        let a_send = a_send.clone();
                        println!(
                            "Initialized track fx param context: {:?} with messages: {:?}",
                            ctx, key_messages
                        );
                        reaper.with_mut(|reaper| {
                            // Track FX Param Name
                            reaper
                                .track_fx_param_name(track_guid, ctx.fx_idx, ctx.param_idx)
                                .bind({
                                    let a_send = a_send.clone();
                                    move |fx_param_name| {
                                        a_send
                                            .try_send(
                                                track::FXParamName {
                                                    track_guid,
                                                    fx_index: ctx.fx_idx,
                                                    param_index: ctx.param_idx,
                                                    name: fx_param_name.param_name.clone(),
                                                }
                                                .into(),
                                            )
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
                                .track_fx_param_value(track_guid, ctx.fx_idx, ctx.param_idx)
                                .bind({
                                    let a_send = a_send.clone();
                                    move |fx_param_value| {
                                        a_send
                                            .try_send(
                                                track::FXParamValue {
                                                    track_guid,
                                                    fx_index: ctx.fx_idx,
                                                    param_index: ctx.param_idx,
                                                    value: fx_param_value.value,
                                                }
                                                .into(),
                                            )
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
                                .track_fx_param_min(track_guid, ctx.fx_idx, ctx.param_idx)
                                .bind({
                                    let a_send = a_send.clone();
                                    move |fx_param_min| {
                                        a_send
                                            .try_send(
                                                track::FXParamMin {
                                                    track_guid,
                                                    fx_index: ctx.fx_idx,
                                                    param_index: ctx.param_idx,
                                                    min: fx_param_min.min,
                                                }
                                                .into(),
                                            )
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
                                .track_fx_param_max(track_guid, ctx.fx_idx, ctx.param_idx)
                                .bind({
                                    let a_send = a_send.clone();
                                    move |fx_param_max| {
                                        a_send
                                            .try_send(
                                                track::FXParamMax {
                                                    track_guid,
                                                    fx_index: ctx.fx_idx,
                                                    param_index: ctx.param_idx,
                                                    max: fx_param_max.max,
                                                }
                                                .into(),
                                            )
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

    let (from_socket_tx, from_socket_rx) = bounded(128); // buffer size as needed

    std::thread::spawn(move || {
        loop {
            println!("Listening on {}", cli.osc_address);
            let mut buf = [0u8; rosc::decoder::MTU];
            match socket.recv_from(&mut buf) {
                Ok((size, _addr)) => {
                    let (_, packet) = rosc::decoder::decode_udp(&buf[..size]).unwrap();
                    from_socket_tx.send(packet);
                    // handle_packet(packet);
                }
                Err(e) => {
                    println!("Error receiving from socket: {}", e);
                    break;
                }
            }
        }
    });

    loop {
        select! {
            recv(from_socket_rx) -> msg => {
            match msg {
                Ok(msg) => {
                    router.dispatch_osc(msg);
                }
                Err(e) => {
                    println!("Error...")
                }
            }
        }
            recv(to_reaper_rx) -> msg => {
                match msg {
                   Ok(track::TrackMsg::Muted(msg))  => {
                        reaper.with_mut(|reaper|{
                            match reaper.track_mute(msg.track_guid).set(TrackMuteArgs{mute: msg.muted}) {
                                Ok(_) => println!("Successfully set mute for track {} to {}", msg.track_guid, msg.muted),
                                Err(e) => println!("Error setting mute for track {}", msg.track_guid),
                            };
                        })
                    }
                    Ok(track::TrackMsg::Soloed(msg)) => {
                        reaper.with_mut(|reaper|{
                            match reaper.track_solo(msg.track_guid).set(generated_osc::TrackSoloArgs{solo: msg.soloed}) {
                                Ok(_) => println!("Successfully set solo for track {} to {}", msg.track_guid, msg.soloed),
                                Err(e) => println!("Error setting solo for track {}", msg.track_guid),
                            };
                        })
                    }
                    Ok(track::TrackMsg::Armed(msg)) => {
                        reaper.with_mut(|reaper|{
                            match reaper.track_rec_arm(msg.track_guid).set(generated_osc::TrackRecArmArgs{rec_arm: msg.armed}) {
                                Ok(_) => println!("Successfully set armed for track {} to {}", msg.track_guid, msg.armed),
                                Err(e) => println!("Error setting armed for track {}", msg.track_guid),
                            };
                        })
                    }
                    Ok(track::TrackMsg::Selected(msg)) => {
                        reaper.with_mut(|reaper|{
                            match reaper.track_selected(msg.track_guid).set(generated_osc::TrackSelectedArgs{selected: msg.selected}) {
                                Ok(_) => println!("Successfully set selected for track {} to {}", msg.track_guid, msg.selected),
                                Err(e) => println!("Error setting selected for track {}", msg.track_guid),
                            };
                        })
                    }
                    Ok(track::TrackMsg::Pan(msg)) => {
                        reaper.with_mut(|reaper|{
                            match reaper.track_pan(msg.track_guid).set(generated_osc::TrackPanArgs{pan: msg.pan}) {
                                Ok(_) => println!("Successfully set pan for track {} to {}", msg.track_guid, msg.pan),
                                Err(e) => println!("Error setting pan for track {}", msg.track_guid),
                            };
                        })
                    }
                    Ok(track::TrackMsg::Volume(msg)) => {
                        reaper.with_mut(|reaper|{
                            match reaper.track_volume(msg.track_guid).set(generated_osc::TrackVolumeArgs{volume: msg.volume}) {
                                Ok(_) => println!("Successfully set volume for track {} to {}", msg.track_guid, msg.volume),
                                Err(e) => println!("Error setting volume for track {}", msg.track_guid),
                            };
                        })
                    }
                    Err(e) => {
                        println!("Error...")}
                    _ => {}
                }
            }
        }
    }
}
