/**
 * @file parser.c
 * @brief UART packet parser with optional CRC16-CCITT and byte-stuffing.
 *
 * Frame format (spec):
 *   [SYNC=0xAA][LEN][TYPE][PAYLOAD...][CHECKSUM]
 * - LEN counts bytes from TYPE through CHECKSUM (inclusive).
 * - 8-bit mode: CHECKSUM = ~((LEN + TYPE + sum(payload)) & 0xFF)
 * - CRC16 mode: two checksum bytes (CRC-CCITT, poly 0x1021, init 0xFFFF),
 *               computed over [LEN, TYPE, PAYLOAD...] (SYNC excluded).
 */

#include "parser.h"
#include <string.h>

/* Feature fallbacks: default ON for CRC and byte-stuffing if not passed by build system. */
#ifndef USE_CRC16
#define USE_CRC16 1
#endif
#ifndef USE_BYTESTUFF
#define USE_BYTESTUFF 1
#endif

enum
{
    S_WAIT_SYNC = 0,
    S_READ_LEN,
    S_READ_TYPE,
    S_READ_PAYLOAD,
#if USE_CRC16
    S_READ_CRC_HI,
    S_READ_CRC_LO
#else
    S_READ_CHK
#endif
};

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

void parser_init(parser_t *p)
{
    memset(p, 0, sizeof(*p));
    p->state = S_WAIT_SYNC;
}

unsigned int parser_invalid_consecutive(const parser_t *p)
{
    return p->invalid_consec;
}

int parser_feed_byte(parser_t *p, uint8_t b, packet_t *out)
{
    /* Allow NULL out pointer (e.g., fuzzing/diff scan that only care about return code). */
    packet_t dummy_out;
    if (out == NULL)
    {
        out = &dummy_out;
    }

#if USE_BYTESTUFF
    /* Unescape (0x7D ^ 0x20) anywhere after SYNC. We must process escape BEFORE state. */
    if (p->esc_next)
    {
        b ^= 0x20;
        p->esc_next = 0;
    }
    else if (b == 0x7D)
    {
        p->esc_next = 1;
        return 0;
    }
#endif

    switch (p->state)
    {
    case S_WAIT_SYNC:
        if (b == SYNC_BYTE)
        {
            p->state = S_READ_LEN;
        }
        else
        {
            p->invalid_consec++;
        }
        return 0;

    case S_READ_LEN:
    {
        if (b < LEN_MIN || b > LEN_MAX)
        {
            p->invalid_consec++;
            p->state = S_WAIT_SYNC;
            return -1;
        }
        p->len = b;
        p->payload_idx = 0;
        p->sum = 0;
        p->state = S_READ_TYPE;
        return 0;
    }

    case S_READ_TYPE:
        p->typ = b;
        p->payload_idx = 0;
        p->sum = 0;
        p->state = S_READ_PAYLOAD;
        return 0;

    case S_READ_PAYLOAD:
    {
        /* LEN = TYPE(1) + PAYLOAD(N) + CHECKSUM(M), M=1 (sum) or 2 (CRC16) */
#if USE_CRC16
        const uint8_t chk_bytes = 2;
#else
        const uint8_t chk_bytes = 1;
#endif
        if (p->len < (uint8_t)(1 + chk_bytes))
        {
            /* impossible length */
            p->invalid_consec++;
            p->state = S_WAIT_SYNC;
            return -1;
        }
        const uint8_t payload_len = (uint8_t)(p->len - (1 + chk_bytes));

        if (p->payload_idx < payload_len)
        {
            if (p->payload_idx < sizeof(p->payload))
            {
                p->payload[p->payload_idx++] = b;
#if !USE_CRC16
                p->sum = (uint16_t)(p->sum + b);
#endif
                return 0;
            }
            else
            {
                /* payload overflow */
                p->invalid_consec++;
                p->state = S_WAIT_SYNC;
                return -1;
            }
        }

        /* No more payload; next is checksum */
#if USE_CRC16
        p->crc_hi = b;
        p->state = S_READ_CRC_LO;
#else
        /* single checksum byte in 8-bit mode */
        p->state = S_READ_CHK;
        return 0;
#endif
        return 0;
    }

#if USE_CRC16
    case S_READ_CRC_LO:
    {
        p->crc_lo = b;
        /* Compute expected CRC over [LEN, TYPE, PAYLOAD...] */
        uint8_t tmp[2 + sizeof(p->payload)];
        size_t t = 0;
        tmp[t++] = p->len;
        tmp[t++] = p->typ;
        memcpy(tmp + t, p->payload, p->payload_idx);
        t += p->payload_idx;

        const uint16_t expected = crc16_ccitt(tmp, t);
        const uint16_t recv = ((uint16_t)p->crc_hi << 8) | p->crc_lo;

        if (recv != expected)
        {
            p->invalid_consec++;
            p->state = S_WAIT_SYNC;
            return -1;
        }

        /* ✅ valid frame */
        p->invalid_consec = 0;
        out->payload_len = p->payload_idx;
        memcpy(out->payload, p->payload, p->payload_idx);

        switch (p->typ)
        {
        case T_START:
            out->type = PKG_START;
            break;
        case T_STOP:
            out->type = PKG_STOP;
            break;
        case T_PING:
            out->type = PKG_PING;
            break;
        case T_RESET:
            out->type = PKG_RESET;
            break;
        default:
            out->type = PKG_UNKNOWN;
            break;
        }

        p->state = S_WAIT_SYNC;
        return 1;
    }
#else
    case S_READ_CHK:
    {
        /* 8-bit checksum: ~((LEN + TYPE + sum(payload)) & 0xFF) */
        const uint8_t expected = checksum8(p->len, p->typ, p->payload, p->payload_idx);
        if (b != expected)
        {
            p->invalid_consec++;
            p->state = S_WAIT_SYNC;
            return -1;
        }

        /* ✅ valid frame */
        p->invalid_consec = 0;
        out->payload_len = p->payload_idx;
        memcpy(out->payload, p->payload, p->payload_idx);

        switch (p->typ)
        {
        case T_START:
            out->type = PKG_START;
            break;
        case T_STOP:
            out->type = PKG_STOP;
            break;
        case T_PING:
            out->type = PKG_PING;
            break;
        case T_RESET:
            out->type = PKG_RESET;
            break;
        default:
            out->type = PKG_UNKNOWN;
            break;
        }

        p->state = S_WAIT_SYNC;
        return 1;
    }
#endif

    default:
        p->state = S_WAIT_SYNC;
        return -1;
    }
}