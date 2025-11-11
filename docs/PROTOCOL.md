# UART Framing Protocol

Each frame follows this structure:
[SYNC=0xAA][LEN][TYPE][PAYLOAD…][CHECKSUM]

## Field Definitions

| Field | Bytes | Description |
|--------|-------|-------------|
| SYNC | 1 | Constant 0xAA start byte |
| LEN | 1 | Number of bytes from TYPE through CHECKSUM inclusive |
| TYPE | 1 | Command identifier |
| PAYLOAD | 0-24 | Optional data |
| CHECKSUM | 1 | ~((LEN + TYPE + sum(payload)) & 0xFF) |

Valid LEN range: 2–32.

### Command Types

| TYPE | Name | Payload | Description |
|------|------|----------|-------------|
| 0x01 | START | none | Enter Active state |
| 0x02 | STOP | none | Return to Idle |
| 0x03 | PING | ASCII bytes | Echo back as PONG |
| 0xFF | RESET | none | Recover from Error state |

---

## Integrity Options

* **Checksum (default)**: simple additive inverse, 8-bit.
* **CRC-16-CCITT**: optional feature flag `USE_CRC16=1`.

---

## Byte-Stuffing

When `USE_BYTESTUFF=1`:

| Byte | Encoded Sequence |
|------|------------------|
| 0xAA | 0x7D 0x8A |
| 0x7D | 0x7D 0x5D |

Decoder reverses transformation before verifying checksum/CRC.

---

## FSM Rules

| Transition | Trigger |
|-------------|----------|
| Idle → Active | START |
| Active → Idle | STOP or 5s inactivity |
| Any → Error | 3 consecutive invalid frames |
| Error → Idle | RESET |

Heartbeat every 1000 ms while Active.