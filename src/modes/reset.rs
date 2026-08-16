use crate::midi::v1m;
use crate::modes::mode_manager::UpstreamIo;

pub fn reset_hardware(io: &mut dyn UpstreamIo, num_channels: usize) {
    for i in 0..num_channels {
        io.send_to_v1m(
            v1m::ChannelFaderMsg {
                idx: i as i32,
                value: 0.0,
            }
            .into(),
        );
        io.send_to_v1m(
            v1m::SelectLEDMsg {
                idx: i as i32,
                state: v1m::LEDState::Off,
            }
            .into(),
        );
        io.send_to_v1m(
            v1m::MuteLEDMsg {
                idx: i as i32,
                state: v1m::LEDState::Off,
            }
            .into(),
        );
        io.send_to_v1m(
            v1m::SoloLEDMsg {
                idx: i as i32,
                state: v1m::LEDState::Off,
            }
            .into(),
        );
        io.send_to_v1m(
            v1m::ArmLEDMsg {
                idx: i as i32,
                state: v1m::LEDState::Off,
            }
            .into(),
        );
        io.send_to_v1m(
            v1m::EncoderRingMsg {
                idx: i as i32,
                mode: v1m::EncoderRingMode::Point,
                val: v1m::ENCODER_CENTER_MODE_CENTER, // Center position
            }
            .into(),
        );
        io.send_to_v1m(
            v1m::TopScribbleStripLine1TextMsg {
                idx: i as i32,
                text: String::new(),
            }
            .into(),
        );
        io.send_to_v1m(
            v1m::TopScribbleStripLine2TextMsg {
                idx: i as i32,
                text: String::new(),
            }
            .into(),
        );
        io.send_to_v1m(
            v1m::TopScribbleStripColorMsg {
                idx: i as i32,
                color: v1m::Color { r: 0, g: 0, b: 0 },
            }
            .into(),
        );
        io.send_to_v1m(
            v1m::BottomScribbleStripLine1TextMsg {
                idx: i as i32,
                text: String::new(),
            }
            .into(),
        );
        io.send_to_v1m(
            v1m::BottomScribbleStripLine2TextMsg {
                idx: i as i32,
                text: String::new(),
            }
            .into(),
        );
    }
}
