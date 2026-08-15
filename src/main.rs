mod osc;
mod shared;
mod traits;

use std::net::{SocketAddrV4, UdpSocket};
use std::str::FromStr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn unix_now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0))
        .as_secs()
}

use clap::Parser;
use crossbeam_channel::{bounded, select};
use midir::{Ignore, MidiInput, MidiInputPort, MidiOutput, MidiOutputConnection};
use rosc::{OscMessage, OscPacket};

use osc::generated_osc;
use osc::generated_osc::{Reaper, context_kind, dispatch_osc};
use osc::route_context::{ContextGateBuilder, OscGatedRouterBuilder};

use arpad_rust::midi::v1m;
use arpad_rust::modes::mode_manager;
use arpad_rust::track::track;

use crate::shared::Shared;
use crate::traits::{Bind, Query, Set};

#[derive(Parser)]
struct Cli {
    #[clap(short, long, default_value = "0.0.0.0:9091")]
    osc_address: String,
    #[clap(short, long, default_value = "0.0.0.0:9090")]
    dev_osc_address: String,
}

fn find_v1m_ports() -> Result<
    (
        MidiInputPort,
        MidiOutputConnection,
        MidiInputPort,
        MidiOutputConnection,
    ),
    Box<dyn std::error::Error>,
> {
    let mut midi_in = MidiInput::new("midir input port sniff")?;
    midi_in.ignore(Ignore::None);
    let midi_out = MidiOutput::new("midir output port sniff")?;
    let midi_out_2 = MidiOutput::new("midir output port sniff")?;

    let mut input_port_1 = None;
    let mut output_port_1 = None;
    let mut input_port_4 = None;
    let mut output_port_4 = None;

    for (i, p) in midi_in.ports().iter().enumerate() {
        if input_port_1.is_none() && midi_in.port_name(p)? == "iCON V1-M Port 1" {
            input_port_1 = Some(p.clone());
            break;
        }
    }
    for p in midi_out.ports().iter() {
        if output_port_1.is_none() && midi_out.port_name(p)? == "iCON V1-M Port 1" {
            output_port_1 = Some(p.clone());
            break;
        }
    }
    for p in midi_in.ports().iter() {
        if input_port_4.is_none() && midi_in.port_name(p)? == "iCON V1-M Port 4" {
            input_port_4 = Some(p.clone());
            break;
        }
    }
    for p in midi_out.ports().iter() {
        if output_port_4.is_none() && midi_out.port_name(p)? == "iCON V1-M Port 4" {
            output_port_4 = Some(p.clone());
            break;
        }
    }

    if input_port_1.is_none() {
        return Err("Could not find V1m MIDI input port".into());
    }

    println!("Found input port {}", input_port_1.clone().unwrap().id());

    if let Some(output_port_1) = output_port_1 {
        println!(
            "Connecting to output port '{}' ...",
            midi_out.port_name(&output_port_1)?
        );
        let output_connection_1 = midi_out.connect(&output_port_1, "midir-test")?;
        if let Some(output_port_4) = output_port_4 {
            let output_connection_4 = midi_out_2.connect(&output_port_4, "midir-test")?;
            Ok((
                input_port_1.unwrap(),
                output_connection_1,
                input_port_4.unwrap(),
                output_connection_4,
            ))
        } else {
            Err("Could not find V1m MIDI output port".into())
        }
    } else {
        Err("Could not find V1m MIDI output port".into())
    }
}

const HEARTBEAT_TIMEOUT_SEC: u64 = 3; // seconds
const WATCHDOG_TICK_MS: u64 = 500; // milliseconds

struct ConnectionState {
    last_heartbeat_s: AtomicU64,
    timed_out: AtomicBool,
    initialized: AtomicBool,
}

impl ConnectionState {
    fn new(now_s: u64) -> Self {
        Self {
            last_heartbeat_s: AtomicU64::new(now_s),
            timed_out: AtomicBool::new(false),
            initialized: AtomicBool::new(false),
        }
    }

    fn on_heartbeat(&self, now_s: u64) -> bool {
        self.last_heartbeat_s.store(now_s, Ordering::Relaxed);
        self.timed_out.store(false, Ordering::SeqCst);
        !self.initialized.swap(true, Ordering::SeqCst) // true if first heartbeat ever
    }

    fn should_timeout(&self, now_s: u64, timeout_s: u64) -> bool {
        now_s.saturating_sub(self.last_heartbeat_s.load(Ordering::Relaxed)) > timeout_s
            && !self.timed_out.swap(true, Ordering::SeqCst)
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
    std::thread::spawn(move || {
        mode_manager::ModeManager::new_from_channels(
            8,
            from_track_manager_rx.clone(),
            to_track_manager_tx.clone(),
            to_mode_manager_rx.clone(),
            from_mode_manager_tx.clone(),
        )
        .run();
    });
    let (input_port_1, output_connection_1, input_port_4, output_connection_4) =
        match find_v1m_ports() {
            Ok(ports) => ports,
            Err(err) => {
                println!("Error finding v1m MIDI ports: {}", err);
                println!("Please ensure v1m is connected and try again.");
                return;
            }
        };
    v1m::V1mBuilder::new(
        input_port_1,
        output_connection_1,
        input_port_4,
        output_connection_4,
        8,
    )
    .build(from_mode_manager_rx.clone(), to_mode_manager_tx.clone());

    let dispatcher = {
        let reaper = reaper.clone();
        move |msg: OscMessage| {
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
                            let track_guid = ctx.track_guid;
                            reaper.track_delete(track_guid).bind({
                                let a_send = a_send.clone();
                                move |_| {
                                    a_send
                                        .try_send(track::Delete { guid: track_guid }.into())
                                        .unwrap();
                                }
                            });
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
                                }
                            });
                            // Track Color
                            reaper.track_color(track_guid).bind({
                                let a_send = a_send.clone();
                                move |color| {
                                    a_send
                                        .try_send(
                                            track::RgbColor {
                                                track_guid,
                                                r: color.r as u8,
                                                g: color.g as u8,
                                                b: color.b as u8,
                                            }
                                            .into(),
                                        )
                                        .unwrap();
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
                    .add_key_route("/track/{guid}/fx/{fx_idx}/name")
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
                                        "Sent FXName for {} fx {} name initial value: {:?}",
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
                                    }
                                });
                            // Track FX Param Value
                            reaper
                                .track_fx_param_value_normalized(
                                    track_guid,
                                    ctx.fx_idx,
                                    ctx.param_idx,
                                )
                                .bind({
                                    let a_send = a_send.clone();
                                    move |fx_param_value| {
                                        a_send
                                            .try_send(
                                                track::FXParamValue {
                                                    track_guid,
                                                    fx_index: ctx.fx_idx,
                                                    param_index: ctx.param_idx,
                                                    value: fx_param_value.value_normalized,
                                                }
                                                .into(),
                                            )
                                            .unwrap();
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

    let connection_state = Arc::new(ConnectionState::new(unix_now_secs()));

    // watchdog thread
    {
        let from_reaper_tx = from_reaper_tx.clone();
        let connection_state = connection_state.clone();
        std::thread::spawn(move || {
            loop {
                std::thread::sleep(Duration::from_millis(WATCHDOG_TICK_MS));
                if connection_state.should_timeout(unix_now_secs(), HEARTBEAT_TIMEOUT_SEC) {
                    from_reaper_tx.send(track::TrackMsg::ResetAll).unwrap();
                    connection_state.initialized.store(false, Ordering::SeqCst);
                    println!(
                        "Heartbeat timeout (>{}). Enqueued ResetAll.",
                        HEARTBEAT_TIMEOUT_SEC
                    );
                }
            }
        });
    }

    let (from_socket_tx, from_socket_rx) = bounded(128); // buffer size as needed

    // socket listening thread
    std::thread::spawn(move || {
        loop {
            let mut buf = [0u8; rosc::decoder::MTU];
            match socket.recv_from(&mut buf) {
                Ok((size, _addr)) => {
                    let (_, packet) = rosc::decoder::decode_udp(&buf[..size]).unwrap();
                    from_socket_tx.send(packet).unwrap();
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
                    Ok(packet) => {
                        if let OscPacket::Message(ref msg) = packet {
                            if msg.addr == "/server/hello" {
                                let first_heartbeat = connection_state.on_heartbeat(unix_now_secs());
                                if first_heartbeat {
                                    reaper.with_mut(|reaper| {
                                        reaper.all_tracks().query().unwrap();
                                    });
                                }
                            } else {
                                router.dispatch_osc(packet);
                            }
                        } else {
                            router.dispatch_osc(packet);
                        }
                    }
                    Err(e) => {
                        println!("Error... {}", e)
                    }
                }
            }
            recv(to_reaper_rx) -> msg => {
                match msg {
                    Ok(track::TrackMsg::Barrier(msg)) => {
                        from_reaper_tx.send(track::TrackMsg::Barrier(msg)).unwrap();
                    }
                    Ok(track::TrackMsg::Muted(msg))  => {
                        reaper.with_mut(|reaper|{
                            match reaper.track_mute(msg.track_guid).set(generated_osc::TrackMuteArgs{mute: msg.muted}) {
                                Ok(_) => {},
                                Err(e) => println!("Error setting mute for track {}", msg.track_guid),
                            };
                        })
                    }
                    Ok(track::TrackMsg::Soloed(msg)) => {
                        reaper.with_mut(|reaper|{
                            match reaper.track_solo(msg.track_guid).set(generated_osc::TrackSoloArgs{solo: msg.soloed}) {
                                Ok(_) => {},
                                Err(e) => println!("Error setting solo for track {}", msg.track_guid),
                            };
                        })
                    }
                    Ok(track::TrackMsg::Armed(msg)) => {
                        reaper.with_mut(|reaper|{
                            match reaper.track_rec_arm(msg.track_guid).set(generated_osc::TrackRecArmArgs{rec_arm: msg.armed}) {
                                Ok(_) => {},
                                Err(e) => println!("Error setting armed for track {}", msg.track_guid),
                            };
                        })
                    }
                    Ok(track::TrackMsg::Selected(msg)) => {
                        reaper.with_mut(|reaper|{
                            match reaper.track_selected(msg.track_guid).set(generated_osc::TrackSelectedArgs{selected: msg.selected}) {
                                Ok(_) => {},
                                Err(e) => println!("Error setting selected for track {}", msg.track_guid),
                            };
                        })
                    }
                    Ok(track::TrackMsg::Pan(msg)) => {
                        reaper.with_mut(|reaper|{
                            match reaper.track_pan(msg.track_guid).set(generated_osc::TrackPanArgs{pan: msg.pan}) {
                                Ok(_) => {},
                                Err(e) => println!("Error setting pan for track {}", msg.track_guid),
                            };
                        })
                    }
                    Ok(track::TrackMsg::Volume(msg)) => {
                        reaper.with_mut(|reaper|{
                            match reaper.track_volume(msg.track_guid).set(generated_osc::TrackVolumeArgs{volume: msg.volume}) {
                                Ok(_) => {},
                                Err(e) => println!("Error setting volume for track {}", msg.track_guid),
                            };
                        })
                    }
                    Ok(track::TrackMsg::SendLevel(msg)) => {
                        reaper.with_mut(|reaper|{
                            match reaper.track_send_volume(msg.track_guid, msg.send_index).set(generated_osc::TrackSendVolumeArgs{volume: msg.level}) {
                                Ok(_) => {},
                                Err(e) => println!("Error setting send volume for track {} send {}", msg.track_guid, msg.send_index),
                            };
                        })
                    }
                    Ok(track::TrackMsg::SendPan(msg)) => {
                        reaper.with_mut(|reaper|{
                            match reaper.track_send_pan(msg.track_guid, msg.send_index).set(generated_osc::TrackSendPanArgs{pan: msg.pan}) {
                                Ok(_) => {},
                                Err(e) => println!("Error setting send pan for track {} send {}", msg.track_guid, msg.send_index),
                            };
                        })
                    }
                    Ok(track::TrackMsg::FXParamValue(msg)) => {
                        reaper.with_mut(|reaper|{
                            match reaper.track_fx_param_value_normalized(msg.track_guid, msg.fx_index, msg.param_index).set(generated_osc::TrackFxParamValueNormalizedArgs{value_normalized: msg.value}) {
                                Ok(_) => {},
                                Err(e) => println!("Error setting fx param value for track {} fx {} param {}", msg.track_guid, msg.fx_index, msg.param_index),
                            };
                        })
                    }
                    Err(e) => {
                        println!("Error...")
                    }
                    _ => {}
                }
            }
        }
    }
}
