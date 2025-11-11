UART Finite State Machine Demo
==============================

Overview
--------
This program simulates and demonstrates a simple UART communication finite state machine (FSM).
It models a byte stream passing through a ring buffer and parser, generating state transitions
and heartbeats based on received packets.

The code is modular and split into several components:

    src/ringbuf.c    – circular buffer implementation
    src/parser.c     – byte stream parser with framing and checksum
    src/fsm.c        – finite state machine for interpreting packets
    src/harness.c    – simulation and demo runner
    src/main.c       – entry point and CLI argument handling

Each component is accompanied by a corresponding header file in src/.

---

Building
--------
Ensure you have `make` and a C compiler (clang or gcc).

From the project root, simply run:

    make

To rebuild from scratch:

    make clean && make

This produces an executable named:

    uart_fsm_demo

---

Running
--------
The program supports several modes, selectable from the command line:

    ./uart_fsm_demo [--test] [--stdin] [--serial <device>]

Descriptions:

1.  **--test**
    Runs two built-in demonstration sequences:
        - A valid sequence: START → PING("hi") → STOP
        - A noise and reset recovery test
    This is the default if no arguments are provided.

    Example:
        ./uart_fsm_demo --test
        or just:
        ./uart_fsm_demo

2.  **--stdin**
    Reads raw bytes from standard input and feeds them into the FSM pipeline.
    This allows piping or redirecting binary files or data streams.

    Example:
        cat data.bin | ./uart_fsm_demo --stdin

3.  **--serial <device>**
    Opens and reads from a serial port device at 115200 baud (8N1, raw mode).
    This mode allows you to observe live FSM behavior from an actual UART device.

    Example:
        ./uart_fsm_demo --serial /dev/tty.usbserial-110

    (Replace the device path with the correct one for your platform.)

---

Output
------
The FSM prints state transitions and responses to stdout, such as:

    STATE: Idle -> Active
    PONG hi
    HEARTBEAT 1000
    HEARTBEAT 2000
    STATE: Active -> Idle (STOP)

These messages show how packets and time-based events drive FSM behavior.

---

Customization
-------------
The behavior of the simulation can be modified by adjusting compile-time macros:

    USE_TICKLESS   – Enables tickless simulation (FSM determines next wake time)
    USE_SERIAL     – Enables serial port support
    USE_CRC16      – Enables CRC16 checks instead of simple checksum
    USE_BYTESTUFF  – Enables byte-stuffing for 0xAA and 0x7D bytes

These can be set in source files or passed to the compiler, for example:

    make clean
    make CFLAGS="-DUSE_TICKLESS=1 -DUSE_SERIAL=1"


Design notes:
 - Ring buffer: fixed 128 bytes, single-producer single-consumer semantics.
 - Parser: internal FSM, LEN range 2..32, PING payload <=24 bytes.
 - Checksum: 8-bit sum of LEN+TYPE+payload, modulo 256, then bitwise NOT:
     chk = ~((LEN + TYPE + sum(payload)) & 0xFF)
 - Resynchronization: non-SYNC bytes while waiting count as invalid attempts;
   invalid consecutive frames tracked and used to transition to Error state.
 - FSM: Idle -> Active on START; Active -> Idle on STOP or 5s inactivity;
   Any -> Error on 3 consecutive invalid frames; Error -> Idle on RESET.
 - Heartbeat: while Active, prints "HEARTBEAT <ms_since_active_start>" every 1000 ms.
 - No dynamic allocation in hot path.

Tests included:
 - Example 1: START -> PING("hi") -> STOP (valid)
 - Example 2: noise + RESET recovery (invalid frames -> Error, then RESET to Idle)