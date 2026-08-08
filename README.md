# Arpad

Arpad is a bridge between hardware control surfaces and recording equipment, especially the Reaper DAW. Arpad communicates with the DAW via a custom OSC (Open Sound Control) protocol, which we have implemented as a [Reaper extension](https://github.com/jdginn/reaper-arpad). (Note: this OSC protocol is different than the default Reaper OSC protocol, which Arpad does not support). Arpad also communicates downstream to hardware control surfaces via MIDI. Generally these surfaces are based on the [Mackie Control Protocol](https://github.com/NicoG60/TouchMCU/blob/main/doc/mackie_control_protocol.md#special-ascii-table-for-assignment-&-timecode-display) (MCU), although like most modern implementations, Arpad supports additional functionality beyond the original MCU specification unique to the various surfaces.

Arpad is designed as a many-to-many bridge, meaning it can connect upstream to one or more DAWs, audio interfaces implementing control via OSC, http, or MIDI, or other equipment and downstream to one or more hardware control surfaces.

## Why Arpad?

We developed Arpad to enable a recording workflow inspired by the workflow on a large-format analog desk. At a philosophical level, we believe that touching faders produces better music faster than using a mouse. We also believe turning knobs creates better-sounding music than staring at a screen, since visual stimulus can make it easy to make decisions based on what one sees, rather than what one hears. It is our belief that the biggest part of the "large format sound" is about workflow -- more than how the desk affects the audio feeding through it. This distinction is more pronounced now that plugins do such a good job of emulating the sound of various analog equipment.

Ideally, Arpad allows the engineer to run a session or mix a song almost entirely from the control surface, only occasionally needing to touch the computer.

One of the attractive aspects of analog desks is that they provide hands-on control of both the recording and mixing process at the same time. Those who have run a session on an analog desk might be familiar with the workflow of putting incoming tracks on the leftmost channels and handling playback of recorded tracks on the right side. This can be a tricky workflow to implement (with zero latency) in a DAW, since the best way to achieve zero latency is to directly control the audio interface. Thus, we wanted to make sure that our solution can seamlessly control both the DAW, the audio interface(s), and any number of other connected devices. Many modern interfaces include an internal near-zero-latency mixer with various effects that can be applied to incoming audio. By using these features, we get most of what a console would provide on the input channel without needing to route the audio through the DAW and incurring corresponding latency.

Similarly, a large format console gives hands-on, immediate control of monitoring, including both monitoring of input tracks and recorded tracks playing back. We wanted to provide the same workflow.

We also wanted to expose a channel strip available at arms-reach, exposing the most important functionality needed to craft a good mix.

A major shortcoming of typical MCU implementations is that even if the features are deep, they are not obviously _discoverable_. The classic MCU implementations expose most of the same functionality as Arpad (although not all!), but they typically require memorizing a complex manual to access those features. (Do I need to press shit or alt to get into sends-on-fader? How do I navigate through the plugins on this track? And so on.) Also, typical MCU implementations do not provide rich enough feedback on the state of the project to make it fully possible to work without looking at the screen. (Which plugin am I controlling again? What plugins are available for this track? And so on.) Arpad aims to make all of this functionality discoverable and obvious. Ideally, if you are familiar with using a mixer, you should be able to use _all_ the functionality in Arpad without needing to read or memorize a manual.

## Why not an existing solution?

Most existing control surface implementations for Reaper are designed as a one-to-one or occasionally one-to-many solution. Most of them do not easily support controlling a recording interface _in concert_ with Reaper.

Most existing implementations use some kind of configuration language or DSL. While these are powerful, they can also be limiting. If you need a feature that has not yet been implemented for the framework, you need to write it before you can use it.

Arpad chose to implement everything in pure rust, but with an API that hopefully makes it trivial to implement most of the features that would otherwise be implemented in a configuration language or a DSL. The types and API are designed to be trivial to plumb and configure even using autocomplete, although by implementing in pure rust, there is always the capability to add specialized functionality. The major tradeoff is that Arpad is only really accessible to users with a software background.

## Why Reaper?

Reaper offers deep scripting and extension support. We can compile our own binaries that extend reaper and the API support is rich. We also like Reaper.

Some other DAWs provide good support for scripting and extension, although most do not. To accomplish our goals, we need deep access to the features of the DAW though extension.

## Who is Arpad for?

We made this for ourselves, and we don't currently plan to implement feature requests for others. Arpad is probably only useful for users with a software background. If that's you, please feel free to fork this repo and have at it!

## Etymology of the name

Árpád Híd is the northernmost bridge over the Danube River in Budapest, Hungary, home of Selah recording studio and the team that wrote this software.

## Why implement in rust?

## Project status

This is still a work in progress with many features incomplete. We do not yet provide many more features than legacy MCU implementations, and many traditional MCU features are still missing from Arpad. Everything is subject to change.

## Architecture

## Features

## Supported Targets

### Supported Upstream Targets:

#### [Reaper](https://www.reaper.fm/) DAW using our [custom OSC extension](https://github.com/jdginn/reaper-arpad).

#### MOTU AVB series audio interfaces (828es, 1248, etc.) using the [MOTU HTTP API](https://motu.com/products/software/motu-datastore/).

Some other DAWs that could be worth considering an implementation for in the future:

- Ableton Live (Javascript API, documented)
- Bitwig (Java API, documented)
- FL Studio (Python API)
- Studio One (JavaScript API but no documentation)
- Cubase (MIDI Remote API, Javascript)
- Ardour (Open Source, so anything is possible with enough effort?)

Some other audio interfaces that could be worth considering an implementation for in the future:

- RME interfaces (internal mixer with effects, good OSC support)
- Metric Halo seems to provide good MIDI support, as well as an internal mixer with effects

### Supported Downstream Targets:

#### iCon Pro Audio V1m, V1x - flagship support in Arpad

#### Behringer X32 - partial support (currently broken, but plan to fix)

## Statement on LLM use

Arpad is primarily designed by hand, sometimes in consultation with an LLM. Code was primarily written by hand, although I did frequently use an LLM to remind me of syntax and suggest style improvements.

An LLM was used to generate some of the early test frameworks (although largely superseded by handwritten tests), as well as some of the more arcane macros. A few earlier commits were contributed agentically by GitHub copilot, although as development has progressed we are no longer using agents to write or commit code.

Parts of the code were generated using an autocomplete in-editor. In general, the types and API are designed to make the trivial plumbing obvious enough for an LLM to easily autocomplete. This is by design. Nontrival aspects are handwritten.

Part of the reason a pure rust implementation works well in 2026 (we believe better than a DSL) is because for a good API, most of the plumbing can be autocompleted. Thus, it's not much more onerous to hook up the various architectural building blocks than using a DSL or configuration language would be. Pure rust is also considerably more flexible.

## Code generation

To generate osc_routes, run the following command:

```bash
cargo run --package=reaper_oscgen
```
