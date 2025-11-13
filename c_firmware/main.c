/**
 * c_firmware/main.c
 * Drive the Rust worker and submit protocol commands. Frames are
 * built inside Rust using uart_fsm_rs::harness::make_frame so the
 * output matches the C reference (CRC16-2B + byte-stuffing when enabled).
 */
#include <stdio.h>
#include <stdint.h>
#include <string.h>
#include <unistd.h>

/* Rust FFI (from rust_logic) */
extern void rust_hello(void);
extern int rust_process_frame(const uint8_t *buf, size_t len);
extern unsigned short rust_crc16(const uint8_t *buf, size_t len);

/* Worker controls */
extern int rust_worker_start(void);
extern int rust_worker_stop(void);

/* New helpers:
 * - rust_make_frame: builds a frame honoring Rust crate features
 * - rust_worker_send_cmd: builds + enqueues a frame inside Rust
 */
extern size_t rust_make_frame(uint8_t typ,
                              const uint8_t *payload, size_t len,
                              uint8_t *out, size_t cap);
extern int rust_worker_send_cmd(uint8_t typ,
                                const uint8_t *payload, size_t len);

static void send_cmd(uint8_t typ, const char *ascii)
{
    const uint8_t *pl = (const uint8_t *)ascii;
    size_t n = ascii ? strlen(ascii) : 0;
    int rc = rust_worker_send_cmd(typ, pl, n);
    if (rc != 0)
    {
        fprintf(stderr, "rust_worker_send_cmd failed (rc=%d)\n", rc);
    }
}

int main(void)
{
    printf("=== C firmware → spawning Rust worker thread ===\n");
    rust_hello();

    if (rust_worker_start() != 0)
    {
        fprintf(stderr, "Failed to start Rust worker\n");
        return 1;
    }

    /* Build & submit START, PING("hi"), STOP via Rust builder */
    send_cmd(0x01, NULL); /* START */
    send_cmd(0x03, "hi"); /* PING */
    send_cmd(0x02, NULL); /* STOP  */

    /* Watch some heartbeats */
    sleep(3);

    rust_worker_stop();
    printf("=== Done ===\n");
    return 0;
}