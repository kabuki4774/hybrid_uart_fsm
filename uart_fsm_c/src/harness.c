/**
 * harness.c
 * @brief Test harness for UART FSM and parser.
 * This file contains functions to simulate feeding byte streams
 * into the UART parser and FSM, including demo scenarios and
 * support for reading from stdin or serial devices.
 *
 * Features:
 * - Demo sequences with valid and invalid packets
 * - Feeding bytes in configurable chunk sizes
 * - Support for tickless mode simulation
 * - Optional serial port reading for live data
 * - Ring buffer integration to simulate ISR behavior
 * - Parser and FSM interaction
 * - Configurable capture mode (not implemented)
 * - Built-in demos for quick testing
 * - Ability to run from stdin or serial device
 *
 * Usage:
 * - Call `run_demos()` to execute built-in demo sequences
 * - Call `run_from_stdin()` to read raw bytes from stdin
 * - Call `run_from_serial(<device>)` to read from a serial device
 *
 * @author Nick Constant + ChatGPT
 * @date 2025-11-11
 */

#include "ringbuf.h"
#include "parser.h"
#include "fsm.h"
#include <stdio.h>
#include <string.h>
#include <unistd.h> /* for usleep on POSIX */

#define USE_TICKLESS 1
#define USE_SERIAL 1

static void feed_bytes_in_chunks(uint8_t *data, size_t len, size_t chunks[], size_t nchunks,
                                 ringbuf_t *rb, parser_t *parser, fsm_t *fsm, uint32_t *now_ms, int capture_mode)
{
    (void)capture_mode;

    size_t cursor = 0;
    size_t chunk_index = 0;
    while (cursor < len)
    {
        size_t cs = chunks[chunk_index % nchunks];
        size_t to_copy = cs;
        if (to_copy > (len - cursor))
            to_copy = len - cursor;
        for (size_t i = 0; i < to_copy; ++i)
        {
            /* simulate ISR: push into ring buffer, drop on overflow */
            (void)rb_push(rb, data[cursor + i]);
        }
        cursor += to_copy;
        chunk_index++;

        /* pump: pop all bytes and feed parser */
        uint8_t b;
        while (rb_pop(rb, &b) == 0)
        {
            packet_t pkt;
            int r = parser_feed_byte(parser, b, &pkt);
            fsm_on_invalid_consecutive(fsm, parser_invalid_consecutive(parser));
            if (r == 1)
            {
                fsm_handle_packet(fsm, &pkt);
            }
        }

/* tick 1 ms per loop for simulation */
#if USE_TICKLESS
        uint32_t next = fsm_next_deadline_ms(fsm);
        if (next == 0)
            next = *now_ms + 1;
        *now_ms = next;
        fsm_tick(fsm, *now_ms);
#else
        (*now_ms)++;
        fsm_tick(fsm, *now_ms);
#endif
    }

    /* continue ticking a bit to allow heartbeats */
    for (int i = 0; i < 3000; ++i)
    {
        (*now_ms)++;
        fsm_tick(fsm, *now_ms);
    }
}

/* helper to build a frame into dest buffer. returns appended length */
static size_t build_frame(uint8_t *dest, size_t cap, uint8_t typ, const uint8_t *payload, size_t payload_len)
{
    if (cap < (size_t)(1 + 1 + 1 + payload_len + 1))
        return 0;
    size_t idx = 0;
    dest[idx++] = SYNC_BYTE;
    uint8_t len = (uint8_t)(2 + payload_len); /* TYPE + payload + CHECKSUM */
    dest[idx++] = len;
    dest[idx++] = typ;
    uint16_t s = len + typ;
    for (size_t i = 0; i < payload_len; ++i)
    {
#if USE_BYTESTUFF
        if (payload[i] == 0xAA || payload[i] == 0x7D)
        {
            dest[idx++] = 0x7D;
            dest[idx++] = payload[i] ^ 0x20;
            continue;
        }
#endif
        dest[idx++] = payload[i];
        s += payload[i];
    }
    uint8_t chk = (uint8_t)(~((uint8_t)(s & 0xFF)));
    dest[idx++] = chk;
    return idx;
}

/* Demo 1 - valid START -> PING("hi") -> STOP */
void demo_valid_sequence(int capture_mode)
{
    printf("=== DEMO 1: valid START -> PING(\"hi\") -> STOP ===\n");
    uint8_t buf[64];
    size_t n1 = build_frame(buf, sizeof(buf), T_START, NULL, 0);
    size_t n2 = build_frame(buf + n1, sizeof(buf) - n1, T_PING, (const uint8_t *)"hi", 2);
    size_t n3 = build_frame(buf + n1 + n2, sizeof(buf) - n1 - n2, T_STOP, NULL, 0);
    size_t total = n1 + n2 + n3;

    ringbuf_t rb;
    rb_init(&rb);
    parser_t parser;
    parser_init(&parser);
    fsm_t fsm;
    fsm_init(&fsm);
    uint32_t now_ms = 0;
    size_t chunks[] = {3, 1, 2, 5};
    feed_bytes_in_chunks(buf, total, chunks, sizeof(chunks) / sizeof(chunks[0]), &rb, &parser, &fsm, &now_ms, capture_mode);
}

/* Demo 2 - noise + RESET recovery */
void demo_noise_reset(int capture_mode)
{
    printf("=== DEMO 2: noise + RESET recovery ===\n");
    uint8_t bytes[] = {0x13, 0x00, 0xAA, 0x01, 0xFF, 0xAA, 0x02, 0xFF, 0xFE};
    size_t total = sizeof(bytes);

    ringbuf_t rb;
    rb_init(&rb);
    parser_t parser;
    parser_init(&parser);
    fsm_t fsm;
    fsm_init(&fsm);
    uint32_t now_ms = 0;
    size_t chunks[] = {2, 2, 4};
    feed_bytes_in_chunks(bytes, total, chunks, sizeof(chunks) / sizeof(chunks[0]), &rb, &parser, &fsm, &now_ms, capture_mode);
}

/* run all built-in demos */
void run_demos(void)
{
    demo_valid_sequence(0);
    demo_noise_reset(0);
}

/* Run using stdin raw bytes (useful for piping) */
void run_from_stdin(void)
{
    ringbuf_t rb;
    rb_init(&rb);
    parser_t parser;
    parser_init(&parser);
    fsm_t fsm;
    fsm_init(&fsm);
    uint32_t now_ms = 0;

    /* Read stdin in blocks, feed into rb, and process loop (simulate ticks) */
    uint8_t tmp[256];
    ssize_t n;
    while ((n = fread(tmp, 1, sizeof(tmp), stdin)) > 0)
    {
        /* push into ringbuf */
        for (ssize_t i = 0; i < n; ++i)
        {
            (void)rb_push(&rb, tmp[i]); /* drop on overflow */
        }
        /* process available bytes */
        uint8_t b;
        while (rb_pop(&rb, &b) == 0)
        {
            packet_t pkt;
            int r = parser_feed_byte(&parser, b, &pkt);
            fsm_on_invalid_consecutive(&fsm, parser_invalid_consecutive(&parser));
            if (r == 1)
                fsm_handle_packet(&fsm, &pkt);
        }
        /* advance some ms */
        now_ms += 10;
        fsm_tick(&fsm, now_ms);
    }
}

#if USE_SERIAL
#include <termios.h>
#include <fcntl.h>
#include <errno.h>
#include <unistd.h>

void run_from_serial(const char *device)
{
    int fd = open(device, O_RDONLY | O_NOCTTY);
    if (fd < 0)
    {
        perror("open");
        return;
    }

    struct termios tio;
    tcgetattr(fd, &tio);
    cfmakeraw(&tio);
    cfsetispeed(&tio, B115200);
    cfsetospeed(&tio, B115200);
    tcsetattr(fd, TCSANOW, &tio);

    ringbuf_t rb;
    rb_init(&rb);
    parser_t parser;
    parser_init(&parser);
    fsm_t fsm;
    fsm_init(&fsm);
    uint32_t now_ms = 0;

    uint8_t buf[128];
    for (;;)
    {
        ssize_t n = read(fd, buf, sizeof(buf));
        if (n <= 0)
        {
            if (errno == EAGAIN || errno == EWOULDBLOCK)
            {
                usleep(10000);
                continue;
            }
            else
                break;
        }
        for (ssize_t i = 0; i < n; ++i)
            rb_push(&rb, buf[i]);
        uint8_t b;
        while (rb_pop(&rb, &b) == 0)
        {
            packet_t pkt;
            int r = parser_feed_byte(&parser, b, &pkt);
            fsm_on_invalid_consecutive(&fsm, parser_invalid_consecutive(&parser));
            if (r == 1)
                fsm_handle_packet(&fsm, &pkt);
        }
        now_ms += 10;
        fsm_tick(&fsm, now_ms);
    }
    close(fd);
}
#endif