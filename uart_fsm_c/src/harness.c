/**
 * harness.c
 * @brief Test harness for UART FSM and parser.
 *
 * Builds spec-correct frames matching compile-time features:
 *  - CRC16 mode: CRC-CCITT over [LEN, TYPE, PAYLOAD...] with 2-byte checksum
 *  - 8-bit mode : ~((LEN + TYPE + sum(payload)) & 0xFF) 1-byte checksum
 * Escapes ALL bytes after SYNC (LEN/TYPE/PAYLOAD/CHECKSUM) when USE_BYTESTUFF=1.
 */

#include "ringbuf.h"
#include "parser.h"
#include "fsm.h"
#include <stdio.h>
#include <string.h>
#include <unistd.h>
#include <stdint.h>

#ifndef USE_TICKLESS
#define USE_TICKLESS 1
#endif
#ifndef USE_SERIAL
#define USE_SERIAL 1
#endif
#ifndef USE_CRC16
#define USE_CRC16 1
#endif
#ifndef USE_BYTESTUFF
#define USE_BYTESTUFF 1
#endif

// --------------- CRC16 helper ---------------
#if USE_CRC16
static uint16_t crc16_ccitt(const uint8_t *buf, size_t len)
{
    uint16_t crc = 0xFFFF;
    for (size_t i = 0; i < len; ++i)
    {
        crc ^= (uint16_t)buf[i] << 8;
        for (int j = 0; j < 8; ++j)
            crc = (crc & 0x8000) ? (crc << 1) ^ 0x1021 : (crc << 1);
    }
    return crc;
}
#else
static uint8_t checksum8(uint8_t len, uint8_t typ, const uint8_t *payload, size_t n)
{
    uint16_t s = (uint16_t)len + (uint16_t)typ;
    for (size_t i = 0; i < n; ++i)
        s = (uint16_t)(s + payload[i]);
    return (uint8_t)(~(s & 0xFF));
}
#endif

/* Byte-stuffed append (escapes 0xAA and 0x7D) */
static inline void append_byte(uint8_t *dest, size_t cap, size_t *idx, uint8_t b)
{
#if USE_BYTESTUFF
    if (b == 0xAA || b == 0x7D)
    {
        if (*idx + 2 <= cap)
        {
            dest[(*idx)++] = 0x7D;
            dest[(*idx)++] = (uint8_t)(b ^ 0x20);
        }
    }
    else
#endif
    {
        if (*idx + 1 <= cap)
        {
            dest[(*idx)++] = b;
        }
    }
}

/**
 * @brief Build a UART frame matching current features.
 * Layout: [AA][LEN][TYPE][PAY...][CHK...] with proper escaping.
 */
static size_t build_frame(uint8_t *dest, size_t cap, uint8_t typ,
                          const uint8_t *payload, size_t payload_len)
{
    if (!dest || cap < 8)
        return 0;

    size_t idx = 0;
    dest[idx++] = SYNC_BYTE; // SYNC (never escaped)

#if USE_CRC16
    const uint8_t len = (uint8_t)(1 /*TYPE*/ + payload_len + 2 /*CRC*/);
#else
    const uint8_t len = (uint8_t)(1 /*TYPE*/ + payload_len + 1 /*CHK*/);
#endif

    // Compute checksum material [LEN, TYPE, PAY...]
    uint8_t tmp[64];
    size_t t = 0;
    tmp[t++] = len;
    tmp[t++] = typ;
    for (size_t i = 0; i < payload_len && t < sizeof(tmp); ++i)
        tmp[t++] = payload[i];

#if USE_CRC16
    const uint16_t crc = crc16_ccitt(tmp, t);
#else
    const uint8_t chk = checksum8(len, typ, payload, payload_len);
#endif

    // Emit escaped bytes after SYNC
    append_byte(dest, cap, &idx, len);
    append_byte(dest, cap, &idx, typ);

    for (size_t i = 0; i < payload_len; ++i)
        append_byte(dest, cap, &idx, payload[i]);

#if USE_CRC16
    append_byte(dest, cap, &idx, (uint8_t)(crc >> 8));
    append_byte(dest, cap, &idx, (uint8_t)(crc & 0xFF));
#else
    append_byte(dest, cap, &idx, chk);
#endif

    return idx;
}

/* Feed bytes in chunks and tick the FSM */
static void feed_bytes_in_chunks(uint8_t *data, size_t len, size_t chunks[], size_t nchunks,
                                 ringbuf_t *rb, parser_t *parser, fsm_t *fsm, uint32_t *now_ms, int capture_mode)
{
    (void)capture_mode;

    size_t cursor = 0, chunk_index = 0;
    while (cursor < len)
    {
        size_t cs = chunks[chunk_index % nchunks];
        size_t to_copy = (cs < (len - cursor)) ? cs : (len - cursor);
        for (size_t i = 0; i < to_copy; ++i)
        {
            (void)rb_push(rb, data[cursor + i]); /* drop on overflow */
        }
        cursor += to_copy;
        chunk_index++;

        /* pump */
        uint8_t b;
        while (rb_pop(rb, &b) == 0)
        {
            packet_t pkt;
            int r = parser_feed_byte(parser, b, &pkt);
            fsm_on_invalid_consecutive(fsm, parser_invalid_consecutive(parser));
            if (r == 1)
                fsm_handle_packet(fsm, &pkt);
        }

        /* tick */
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

    /* allow heartbeats to appear */
    for (int i = 0; i < 3000; ++i)
    {
        (*now_ms)++;
        fsm_tick(fsm, *now_ms);
    }
}

/* Demo 1 - valid START -> PING("hi") -> STOP */
void demo_valid_sequence(int capture_mode)
{
    printf("=== DEMO 1: valid START -> PING(\"hi\") -> STOP ===\n");
    uint8_t buf[128];
    size_t n = 0;
    n += build_frame(buf + n, sizeof(buf) - n, T_START, NULL, 0);
    n += build_frame(buf + n, sizeof(buf) - n, T_PING, (const uint8_t *)"hi", 2);
    n += build_frame(buf + n, sizeof(buf) - n, T_STOP, NULL, 0);

    ringbuf_t rb;
    rb_init(&rb);
    parser_t parser;
    parser_init(&parser);
    fsm_t fsm;
    fsm_init(&fsm);
    uint32_t now_ms = 0;
    size_t chunks[] = {3, 1, 2, 5};
    feed_bytes_in_chunks(buf, n, chunks, sizeof(chunks) / sizeof(chunks[0]),
                         &rb, &parser, &fsm, &now_ms, capture_mode);
}

/* Demo 2 - noise + RESET recovery (now uses a VALID RESET frame) */
void demo_noise_reset(int capture_mode)
{
    printf("=== DEMO 2: noise + RESET recovery ===\n");

    uint8_t buf[128];
    size_t n = 0;

    /* Noise & short/bad frame to trigger 3 invalids */
    const uint8_t noise[] = {0x13, 0x00, 0xAA, 0x01, 0xFF};
    memcpy(buf + n, noise, sizeof(noise));
    n += sizeof(noise);

    /* Valid RESET frame (matches features) */
    n += build_frame(buf + n, sizeof(buf) - n, T_RESET, NULL, 0);

    ringbuf_t rb;
    rb_init(&rb);
    parser_t parser;
    parser_init(&parser);
    fsm_t fsm;
    fsm_init(&fsm);
    uint32_t now_ms = 0;
    size_t chunks[] = {2, 2, 4};
    feed_bytes_in_chunks(buf, n, chunks, sizeof(chunks) / sizeof(chunks[0]),
                         &rb, &parser, &fsm, &now_ms, capture_mode);
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

    uint8_t tmp[256];
    size_t n;
    while ((n = fread(tmp, 1, sizeof(tmp), stdin)) > 0)
    {
        for (size_t i = 0; i < n; ++i)
            (void)rb_push(&rb, tmp[i]);

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
            (void)rb_push(&rb, buf[i]);

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