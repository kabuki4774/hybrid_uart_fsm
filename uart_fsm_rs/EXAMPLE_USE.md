# UART FSM Simulator — Quick Command Cheat Sheet

This doc summarizes the common commands to simulate UART traffic between two virtual TTYs and your FSM app.

---

## 1) Create a virtual serial pair (PTYs)

Writer → `/tmp/ttyV1`, Reader ← `/tmp/ttyV0`

```bash
socat -d -d pty,raw,echo=0,link=/tmp/ttyV0 pty,raw,echo=0,link=/tmp/ttyV1 & SOCAT_PID=$!
# later: kill $SOCAT_PID
```
**Flags**
- `-d -d` — verbose logs
- `raw,echo=0` — binary mode, no echo
- `link=...` — stable paths to reference from other tools

---

## 2) Run the simulator (read from `/tmp/ttyV0`)

```bash
cargo run --no-default-features --features std,demo_tty -- /tmp/ttyV0
```
> `std` enables the binary; `demo_tty` selects the TTY-reading main that feeds bytes into `run_demo`.

---

## 3) Generate frames (example program)

### Raw bytes to stdout (for piping)
```bash
# START → raw bytes
cargo run --quiet --example make_frame -- --raw START | socat -u - /tmp/ttyV1

# PING "hi" → raw bytes
cargo run --quiet --example make_frame -- --raw "PING hi" | socat -u - /tmp/ttyV1

# STOP → raw bytes
cargo run --quiet --example make_frame -- --raw STOP | socat -u - /tmp/ttyV1
```
> `--` separates Cargo’s flags from your example’s args.  
> `--raw` makes the example write only the frame bytes to stdout.

### Inspect frame bytes without sending
```bash
cargo run --quiet --example make_frame -- --raw START | hexdump -C
```

---

## 4) Scripted sequences

### Timed sequence (simulate gaps)
```bash
( cargo run --example make_frame START --quiet; sleep 1;   cargo run --example make_frame "PING hi" --quiet;   sleep 1;   cargo run --example make_frame STOP --quiet ) | cargo run
```

### Inspect & forward (tee to hex viewer)
```bash
cargo run --example make_frame "PING hi" | tee >(xxd) | cargo run
```

---

## 5) Feature alignment (important)

Frames **must** use the same features as the simulator (e.g., `byte_stuff`, `crc16`).

```bash
# Simulator with byte_stuff
cargo run --no-default-features --features std,demo_tty,byte_stuff -- /tmp/ttyV0

# Sender with byte_stuff (matches parser)
cargo run --quiet --no-default-features --features std,byte_stuff   --example make_frame -- --raw START | socat -u - /tmp/ttyV1
```
Mismatch ⇒ “invalid frames”.

---

## 6) Quick troubleshooting

- **Seeing**: `ERRORS: 3 invalid frames`  
  **Likely**: you sent text (forgot `--raw`) or feature mismatch.

- **Verify bytes**:
```bash
cargo run --quiet --example make_frame -- --raw START | hexdump -C
hexdump -C /tmp/ttyV0
```

- **Silence cargo noise**: add `--quiet` before `--example`.

---

## 7) Handy notes

- PTY writer: `/tmp/ttyV1` (send frames here)  
- PTY reader: `/tmp/ttyV0` (your simulator reads here)  
- Stop `socat`: `kill $SOCAT_PID`  
- The binary only builds with `std`. Use `--features std,<mode>` (e.g., `demo_tty`, `demo_file`, `demo_sim`).
