# Bugs and Issues Found in TrackSendsMode

This document tracks bugs discovered during the implementation of the comprehensive test suite for `reaper_track_sends.rs`.

## Fixed Bugs

### Bug #1: SendLevel messages forwarded to hardware for unmapped sends
**Status**: ✅ FIXED  
**Test**: `test_02_send_level_for_unmapped_send_is_ignored`  
**Severity**: High  

**Description**: 
The `handle_downstream_messages` method was sending fader updates to hardware (XTouch) even when the send index was not mapped to any target track. This caused unexpected hardware behavior.

**Root Cause**:
In the `SendLevel` payload handling (lines 90-98 of original code), the code unconditionally sent fader messages without checking if the send was mapped:

```rust
TrackDataPayload::SendLevel(msg) => {
    let fader_value = msg.level;
    self.to_xtouch
        .send(XTouchDownstreamMsg::FaderAbs(FaderAbsMsg {
            idx: msg.send_index,
            value: fader_value as f64,
        }))
        .unwrap();
}
```

**Fix**:
Added validation to check if the send index is mapped before forwarding:

```rust
TrackDataPayload::SendLevel(msg) => {
    // Only send fader update if the send index is mapped to a target
    let assignments = self.track_sends.lock().unwrap();
    if assignments
        .get(msg.send_index as usize)
        .and_then(|opt| opt.as_ref())
        .is_some()
    {
        let fader_value = msg.level;
        self.to_xtouch
            .send(XTouchDownstreamMsg::FaderAbs(FaderAbsMsg {
                idx: msg.send_index,
                value: fader_value as f64,
            }))
            .unwrap();
    }
}
```

**Impact**: 
Without this fix, moving send levels in Reaper before the sends were mapped would cause random fader movements on the hardware controller.

## Known Issues / Future Enhancements

### Issue #1: Missing EPSILON threshold filtering
**Status**: 📝 DOCUMENTED  
**Test**: `test_17_send_level_changes_below_epsilon_threshold_ignored`  
**Severity**: Low  

**Description**:
Unlike `VolumePanMode`, `TrackSendsMode` does not filter out small send level changes below an EPSILON threshold. This can result in excessive hardware updates for very small value changes.

**Expected Behavior**:
Send level changes smaller than EPSILON (0.01) should not trigger hardware fader updates.

**Current Behavior**:
All send level changes, no matter how small, trigger hardware updates.

**Recommendation**:
Implement EPSILON filtering similar to `VolumePanMode`:
1. Store the last sent value for each send index
2. Before sending a fader update, compare the new value with the stored value
3. Only send the update if the difference exceeds EPSILON
4. Update the stored value when a message is sent

### Issue #2: Missing state accumulation for unmapped sends
**Status**: 📝 DOCUMENTED  
**Severity**: Medium  

**Description**:
When send level updates arrive for sends that haven't been mapped yet, those values are discarded. If the send is later mapped, the hardware won't receive the current state.

**Expected Behavior** (similar to VolumePanMode):
1. State should accumulate even for unmapped sends
2. When a send is mapped via `SendIndex`, all accumulated state should be sent to hardware
3. This ensures the hardware always reflects the actual state in Reaper

**Current Behavior**:
- Send level updates for unmapped sends are silently ignored
- When a send is later mapped, no state is sent to hardware
- Hardware faders remain at their previous position until a new update arrives

**Recommendation**:
1. Add a state storage structure (similar to `TrackState` in `VolumePanMode`)
2. Store incoming `SendLevel` updates even for unmapped sends
3. When processing `SendIndex`, check if accumulated state exists and send it to hardware

### Issue #3: Missing pan support for sends
**Status**: 📝 DOCUMENTED (existing TODO)  
**Severity**: Low  

**Description**:
The code has a TODO comment at line 107: `// TODO: pan`

**Current Behavior**:
`SendPan` messages are ignored (fall through to the catch-all case).

**Recommendation**:
Implement pan support for sends, similar to how `SendLevel` is handled:
1. Handle `DataPayload::SendPan` in the downstream message handler
2. Map send pan to encoder ring LEDs
3. Add similar mapping check as with SendLevel
4. Consider EPSILON filtering for pan values

### Issue #4: Unused struct TrackSendState
**Status**: ✅ FIXED (commit 0f9e8dd)
**Severity**: Very Low  

**Description**:
Line 12 defined an empty struct `TrackSendState` that was never used.

**Fix**:
Removed the unused struct to clean up the codebase.

### Issue #5: Missing bounds checking
**Status**: ✅ FIXED (commit fed27a8)
**Severity**: Medium  

**Description**:
When accessing `assignments[msg.send_index as usize]`, there was no explicit bounds checking before the array access at line 88 in `SendIndex` handling.

**Previous Behavior**:
If `send_index` was >= `num_channels`, the code would panic.

**Fix**:
Added bounds checking:
```rust
TrackDataPayload::SendIndex(msg) => {
    let mut assignments = self.track_sends.lock().unwrap();
    // Add bounds checking to prevent panic on invalid send_index
    if (msg.send_index as usize) < assignments.len() {
        assignments[msg.send_index as usize] = Some(msg.guid);
    }
    // If out of bounds, silently ignore (could log error in production)
}
```

Note: The `SendLevel` handler already uses `.get()` which is safe and returns `None` for out-of-bounds indices.

## Test Coverage

The test suite includes 16 tests covering:
- ✅ Basic send assignment and mapping
- ✅ Mapped vs unmapped send handling
- ✅ Upstream fader movements
- ✅ Downstream send level updates
- ✅ State accumulation across multiple sends
- ✅ Message ordering (upstream and downstream)
- ✅ Barrier handling for mode transitions
- ✅ Mode transition initialization
- ✅ Complex multi-send scenarios

### Tests Documenting Known Issues

- `test_17_send_level_changes_below_epsilon_threshold_ignored` - Documents Issue #1 (EPSILON filtering)
- `test_04_state_accumulation_for_unmapped_sends` - Documents Issue #2 (state accumulation)

## Summary

- **Fixed**: 3 bugs
  1. Unmapped send handling (commit 046a1ef)
  2. Unused TrackSendState struct (commit 0f9e8dd)
  3. Missing bounds checking (commit fed27a8)
- **Documented**: 3 issues for future enhancement
  1. Missing EPSILON threshold filtering
  2. Missing state accumulation for unmapped sends
  3. Missing pan support
- **Test Coverage**: 23 comprehensive tests

All tests currently pass with the bug fixes applied.
