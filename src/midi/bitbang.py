#!/usr/bin/env python3
"""
Send raw MIDI hex messages to a named MIDI port.

Usage:
    python midi_send.py "Port Name" "ef 7f 7f"
    python midi_send.py "Port Name" "90 3c 7f"   # note on, middle C, velocity 127
    python midi_send.py --list                    # list available output ports
"""

import sys
import rtmidi
import time


def list_ports():
    midi_out = rtmidi.MidiOut()
    ports = midi_out.get_ports()
    if not ports:
        print("No MIDI output ports found.")
    else:
        print("Available MIDI output ports:")
        for i, name in enumerate(ports):
            print(f"  [{i}] {name}")


def find_port_by_name(midi_out, name):
    ports = midi_out.get_ports()
    for i, port_name in enumerate(ports):
        if name.lower() in port_name.lower():
            return i, port_name
    return None, None


def send_hex(port_name, hex_string):
    midi_out = rtmidi.MidiOut()

    index, matched_name = find_port_by_name(midi_out, port_name)
    if index is None:
        print(f"Error: No port matching '{port_name}' found.")
        print()
        list_ports()
        sys.exit(1)

    print(f"Opening port: {matched_name}")
    midi_out.open_port(index)

    try:
        raw_bytes = bytes.fromhex(hex_string.replace(" ", ""))
        message = list(raw_bytes)
        print(f"Sending: {hex_string.strip()}")
        midi_out.send_message(message)
    except ValueError as e:
        print(f"Error parsing hex string '{hex_string}': {e}")
        sys.exit(1)
    finally:
        midi_out.close_port()


if __name__ == "__main__":
    # Not sure what this does
    send_hex("V1-M Port 4", "ef 7f 7f")

    # ------------
    # Setting touchscreen tile one to some random text
    # ------------
    # send_hex(
    #     "V1-M Port 4",
    #     f"F0 1D 03 10 09 26 01 01 00 0C 02 08 37 38 39 60 40 40 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 F7",
    # )
    send_hex(
        "V1-M Port 4",
        "F0 1D 03 10 09 26 01 01 08 0c 02 08 31 32 33 34 35 36 37 38 39 40 41 42 43 44 45 F7",
    )
    # NOTE IMPORTANT: do NOT need to save updated settings to ROM for them to update!
    # However, do not see updates until switching DAW and slot (ec 22 XX, see below)
    #
    # Formatted as:
    # [slot] [daw id] [part #] [n buttons] [n lines] [bytes per line] [text...]
    #
    # Observations:
    # - Setting to 1 button, one line, 1 bytes sets a single character but seems to leave a bogus character after
    # - Setting to 1 button, one line, more bytes seems to work fine
    # - When you don't set the WHOLE touchscreen, it appears to leave a cursor behind
    # - Seting to fewer bytes doesn't seem to matter much
    # - Changing # buttons or # line sappears to have no effect
    # - PART # REALLY MATTERS
    #
    # This needs further investigation but it kind of seems like part ends up deciding whether we are on the top or bottom row of the touchscreen. The other two numbers don't seem to matter as much.
    # part 0 -> blue layer
    # part 3 -> bottom of green layer
    # part 4 -> top of yellow layer
    # part 6 -> top of U1
    # part 8 -> top of U2
    #
    # TAKEAWAYS SO FAR:
    # 1. We can relabel touch keys at-will, without having to touch the ROM
    # 2. We can relable keys on any function layer
    # 3. We can also remap the behavior of the keys but it's probably easier for us to leave them all the same and route them in our own software

    # ------------
    # ------------
    # Switching DAW and slot
    # ------------
    # send_hex("V1-M Port 4", "ec 22 26")
    # Reaper is 26 / 46 / 66
    # ec 22 21 -> bitwig slot 1
    # ec 22 41 -> bitwig slot 2
    # ec 22 61 -> bitwig slot 3
    # ec 22 22 -> switch to Cubase on slot 1
    # 23 protools
    # 32 luna
    # 33 user 2
    # 34 user 3
    # 3a -> Cakewalk on slot 1
    # 4a -> Cakewalk on slot 2
    # 6a -> Cakewalk on slot 3
    # -------

    # ------------
    # MacOS: set last bit to 1
    # V1M replies with 9a 2a 01
    # -----------
    send_hex("V1-M Port 4", "ec 2a 01")

    # ------------------------------------------------------

    # ------------
    # Setting DAW for slot
    # -----------
    # No effect: send_hex("V1-M Port 4", "ec 2e 22")
    # send_hex(
    #     "V1-M Port 4", "f0 1d 03 10 09 2e 22 00 f7"
    # )  # Observed 22 for Reaper, 28 for StudioOne, but unsure what this does
    # ------------

    #
    # send_hex("V1-M Port 4", "ec 2e 03")
    # send_hex("V1-M Port 4", "ec 22 68")
    # send_hex("V1-M Port 4", "ef 7f 7f")
    # send_hex("V1-M Port 4", "F0 1D 03 10 09 24 00 00 00 00 00 00 00 00 00")
    # send_hex("V1-M Port 4", "ef 7f 7f")
    # send_hex("V1-M Port 4", "ec 2a 01")
    # send_hex("V1-M Port 4", "F0 1D 03 10 09 2C 21 04 F7")
    # send_hex(
    #     "V1-M Port 4",
    #     "F0 1D 03 10 09 25 21 00 08 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 F7",
    # )
    # send_hex(
    #     "V1-M Port 4",
    #     f"F0 1D 03 10 09 26 01 01 00 0C 02 08 44 44 65 54 40 40 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 20 F7",
    # )
    # Pro tools mode only
    # send_hex("V1-M Port 1", "f0 00 00 66 05 00 12 00 30 31 32 33 34 35 36 37 38 39 f7")
    # send_hex("V1-M Port 1", "f0 00 00 66 05 00 12 08 30 31 32 33 34 35 36 37 38 39 f7")
    # Main display
    # send_hex("V1-M Port 1", "f0 00 00 66 14 12 08 30 31 32 33 34 35 36 37 38 39 f7")
    # Save to ROM
    # send_hex("V1-M Port 4", "ec 2d 01")
    # send_hex("V1-M Port 4", "ef 7f 7f")
    # Switch to slot 2
    # send_hex("V1-M Port 4", "ec 22 42")
    # Switch back to slot 1
    # send_hex("V1-M Port 4", "ec 22 22")
