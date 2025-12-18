Arpad is implemented as a message-passing pipeline that translates and passes messages between endpoints that communicate with the outside world.

Endpoints are referred to as Upstream or Downstream depending on whether they are connected to some software or device that is processing audio (e.g. Reaper or some other DAW, an audio interface) or are connected to some device that is designed to provide control over an audio processing device (e.g. a MIDI controller, fader controller). Endpoints are EITHER upstream or downstream. The endpoint is always the furthest-upstream or furthest-downstream worker in the pipeline. Note that it is possible to have multiple upstream or downstream endpoints at a time (although we have not yet implemented such a configuration.)

A key concept in the operation of Arpad is that of _modes_. A mode is a mapping of some set of the inputs/outputs available on an upstream endpoint to a downstream endpoint. Modes are exclusive. Transitions between modes require keeping track of state, since we must accumulate all changes from upstream that would apply to any inactive modes. Also, it is important that control from a downstream endpoint is only enabled once a mode transition is finished (i.e. the control hardware matches the state of the upstream endpoints) to ensure that stale physical state on the controller cannot send spurious or conflicting messages upstream.

Endpoints communicate with the interior workers via channels. Some interior workers manage state, some manage modes. The most important interior worker is the ModeManager, which is responsible for tracking the current mode, managing transitions between modes, and ensuring that state is synchronized between upstream and downstream endpoints during mode transitions. Interior workers can be described as being upstream or downstream of each other.

Endpoints should not need to know about the existence of modes, nor should any other interior workers.

Here is a rough diagram of the architecture as it is currently:

```
[Upstream]                                                                   [Downstream]

[reaper (over OSC)] <--> [TrackStateManager] <--> [ModeManager] <--> [XTouch (over midi)]

- TrackStateManager maintains the current state of tracks in reaper based on all messages we have ever received from reaper. This avoids us having to make exhaustive requests of state from reaper during mode transition. We want to avoid burdening reaper with unnecessary requests, since it is performance-critical.

```
