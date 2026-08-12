# Channel Strip Mode Design

Implements a mode where the faders and Arm/Mute/Solo/Select buttons behave the same as VolumePanMode
but the encoders and scribble strpes expose key tone-shaping functions like EQ, Compression, Saturation, etc.

## Principles

- Everything fits on the top scribble strip
- Everything is iether visible or visible by ONLY pressing Shift
- Shift does not change anything unless you turn/click an encoder
- An encoder EITHER has a Pressed (press&turn) functionality OR a Click functionality but never both
- Top line of scribble strip displays what happens on a Normal turn (i.e. no press)
- "Encoer ring" (shown as a bar on v1) displays the value of the Normal turn parameter
- On turn, top line shows precise value while turning and for 2 seconds after the turn stops, then reverts to showing the parameter name
- Bottom line shows Pressed turn OR Click _value_
- Top and bottom line are both related to the same element
- Bottom line values should always make it obvious what the value means, when seen below the top line
- Colors match element
- Shfit _may_ EITHER switch a widget to a different element or expose extra parameters of the
  non-shift element
- Shift elements should be logically related to non-shift elements on the same widget
- Everything for channel strip mode should fit on ONLY encoders + shift
- Shift behavior is a HOLD, not a toggle

The encoders ONLY control the selected track, and only control one track at a time. Track
selection still follows reaper and still responds to the select buttons on the surface.

## Encoder behaviors

Encoders support multiple behaviors, with each encoder supporting up to the following:

1.  Turn the encoder without pressing anything
2.  Turn the encoder WHILE holding it down
3.  Turn the encoder WHILE holding down a modifier button (e.g., Shift)
4.  Turn the encoder WHILE holding down both the modifier AND pressing down the encoder
5.  Click the encoder (turning does nothing)
6.  Click the encoder WHILE holding down a modifier button (e.g., Shift -- turning does nothing)

In each case, the behavior updates the scribble strip to indicate what parameter is being controlled and the encoder ring to indicate the current value.

This mode assumes 16 encoders are available. The encoders have the following functions:

## Controls in each mode

<!-- markdownlint-disable MD060 -->

| #   | Normal      | Pressed                                  | Shift           | Shift+Pressed | Click          | Shift+Click     |
| --- | ----------- | ---------------------------------------- | --------------- | ------------- | -------------- | --------------- |
| 1   | HP filter   | slope                                    | EQ type         |               |                |                 |
| 2   | Low freq    | Low Q (bell) / slope (shelf)             | bell/shelf      |               |                |                 |
| 3   | Low gain    |                                          |                 |               | zero Low gain  |                 |
| 4   | LM freq     | LM Q                                     |                 |               |                |                 |
| 5   | LM gain     |                                          |                 |               | zero LM gain   |                 |
| 6   | HM freq     | HM Q                                     |                 |               |                |                 |
| 7   | HM gain     |                                          |                 |               | zero HM gain   |                 |
| 8   | High freq   | High Q (bell) / slope (shelf) / Q (filt) | bell/shelf/filt |               |                |                 |
| 9   | High gain   |                                          | sides gain      |               | zero High gain | zero sides gain |
| 10  | EQ pos      |                                          | Comp order      |               | bypass EQ      |                 |
| 11  | Comp thresh | Comp SC filter                           | Comp2 thresh    | Comp2 SC filt |                |                 |
| 12  | Comp ratio  | Comp attack                              | Comp2 ratio     | Comp2 attack  |                |                 |
| 13  | Comp makeup | Comp release                             | Comp2 makeup    | Comp2 release |                |                 |
| 14  | Comp type   |                                          | Comp2 type      |               | bypass Comp    | bypass Comp2    |
| 15  | Saturation  |                                          | Saturation type |               | bypass Sat     |                 |
| 16  | Gain        | Interface gain (only if armed)           | Trim            |               |                |                 |

Notes on specific controls:

- By default, EQ is engaged, both compressors and saturation are bypassed.
- EQ type selects between EQ plugins with EQUIVALENT features. It may allow e.g. colourless EQ, SSL-style, Neve-style, etc.
- Depending on EQ type, Q may or may not take effect.
- Sides gain applies the "High" band only to the sides in a mid-side EQ. This value is offset from the main high gain.
- EQ pos sets the position of EQ in the signal chain. Modes:
  - "FIRST": Gain -> EQ -> Comp -> Comp -> Saturation -> Trim
  - "BET": Gain -> Comp -> EQ -> Comp -> Saturation -> Trim
  - "MIDDLE": Gain -> Comp -> Comp -> EQ -> Saturation -> Trim
  - "LAST": Gain -> Comp -> Comp -> Saturation -> EQ -> Trim
- Comp order sets the ordering of compressors. Modes:
  - "Cmp1->2"
  - "Cmp2->1"
- Comp and Comp2 are separate compressors and controlled fully independently.
- Comp is a "fast", FET-style compressor. Comp2 is a "slow" optical-style compressor.
- Comp type selects between compressors germane to the two categories above. Examples:
  - Comp1: 1176, Distressor, Digital, SSL, API
  - Comp2: LA2A, LA3A, Vari-MU, etc.
  - NOTE: maybe we want Comp1 and Comp2 to both support all types?
- Comp controls vary based on type
- Comp SC filter is a high-pass filter on the compressor sidechain.
- Saturation type selects between various console, tape simulators up to full-on distortion.
- Gain adjusts level entering the channel strip, before any processing.
- Trim adjust level leaving the channel strip.
- Interface gain adjusts the gain at the audio interface, if the selected tack is armed. This does not affect recorded material.

## Colors by element:

<!-- markdownlint-disable MD060 -->

| Color        | Element    | Notes                            |
| ------------ | ---------- | -------------------------------- |
| Black        | Meta       | Eq type, Eq position, Comp order |
| Dark Brown   | HPF        |                                  |
| Brown        | Low        |                                  |
| Dark Blue    | LM         |                                  |
| Green        | HM         |                                  |
| Red          | High       |                                  |
| Light Orange | LPF        |                                  |
| Pink         | Eq sides   |                                  |
| Amber        | Comp 1     |                                  |
| White        | Comp 2     |                                  |
| Orange       | Saturation |                                  |
| Purple       | Interface  |                                  |
| Grey         | Gain/Trim  |                                  |
| Light Green  | Delay      |                                  |
| Sky Blue     | Reverb     |                                  |

## Scribble Strip definitions/examples

Note: scribble strip is always exactly 7 characters wide.

<!-- markdownlint-disable MD060 -->

| #   | Normal L1 | Turn L1   | Normal 2  | Shift L1 | S+Turn L1 | Shift L2  | Notes                                                                                                 |
| --- | --------- | --------- | --------- | -------- | --------- | --------- | ----------------------------------------------------------------------------------------------------- |
| 1   | HpfFreq   | 150Hz     | -12/oct   | EqType   |           | SSL\*     | Options: SSL, Neve, API, Digital, etc.                                                                |
| 2   | LowFreq   | 300Hz     | 0.8 Q\*   | LowMode  |           | bell\*    | Options: bell, shelf; for shelf mode, don't display Q on L                                            |
| 3   | LowGain   | -5db      | zero      | LowGain  | -5db      | zero      |                                                                                                       |
| 4   | LM Freq   | 400Hz     | 1.0 Q     | LM Freq  | 400Hz     | 1.0 Q     |                                                                                                       |
| 5   | LM Gain   | 3db       | zero      | LM Gain  | 3db       | zero      |                                                                                                       |
| 6   | HM Freq   | 2000Hz    | 1.0 Q     | HM Freq  | 2000Hz    | 1.0 Q     |                                                                                                       |
| 7   | HM Gain   | 5db       | zero      | HM Gain  | 5db       | zero      |                                                                                                       |
| 8   | Hi Freq   | 5000Hz    | 1.0 Q\*   | Hi Mode  |           | bell\*    | Options: bell (1.0 Q), shelf (blank), filter (0.72 Q)                                                 |
| 9   | Hi Gain   | 0db\*     | zero      | SidesGn  | 2db       | zero      | bell (5db), shelf (5db), filter (-12/oct)                                                             |
| 10  | EqFirst\* | E>C>C>S\* | EqIN\*    | CmpOrdr  | Cmp1->2   |           | Options: EqFirst, EqMiddl, EqLast; E>C>C>S, C>E>C>S, C>C>E>S>, C>C>S>E; EqIN, EqOut; Cmp1->2, Cmp2->1 |
| 11  | CompThr\* | -20db     | 10msAtk\* | Cmp2Thr  | -20db     | 10msAtk\* | CmpThr has different behavior depending on CompType                                                   |
| 12  | CompRat\* | 3:1       | 90msRel\* | Cmp2Rat  | 3:1       | 90msRel\* | Attack, relase time sometimes in us for some compressors                                              |
| 13  | CompMkp\* | +5db      | 200HzSC\* | Cmp2Mkp  | -5db      | 200HzSC\* | Sc Freq blank for no sidechain filter (turn all the way left)                                         |
| 14  | 1176\*    |           | CompIN    | LA2A\*   |           | Cmp2IN    | Options: 1176, LA2A, Digital, SSL, Distressor, Vari-MU, etc.                                          |
| 15  | Sat       |           | SatIN\*   | Sat      |           | Tape\*    | Options: Tape, Console, Distortion, etc.                                                              |
| 16  | Gain      | -12db     | Intrfc\*  | Trim     |           | -12db     | Intrfc blank if track not armed; Interface gain on turn                                               |

## Scribble mappings for compressor types:

### 1176

- CompThr -> CmpInpt, Cmp2Inp
- CompMkp -> CmpOtpt, Cmp2Otp

### LA2A, LA3A

- CompThr -> CmpGain, Cmp2Gn
- CompRat -> CmpRedn, Cmp2Red

### Distressor

- CompThr -> CmpInpt, Cmp2Inp
- CompMkp -> CmpOtpt, Cmp2Otp

### Fairchild

- CmpRat -> CmpInpt
- CmpAtk -> TmCnst<1-5>

## Ideas if we add more V1x:

- Support additional FX elements
  - Pultec? (separate from channel EQ)
  - Surgical EQ?
  - Delay?
  - Reverb?
  - Vibrato/Chorus etc.?
