/**
 * @file parser.c
 * @brief Implementation of the UART packet parser.
 * This file contains the implementation of a byte-wise parser
 * for a simple UART protocol with framing, checksums,
 * and packet types.
 *
 * Features:
 * - Byte-wise feeding of data
 * - State machine for parsing frames
 * - Checksum validation
 * - Handling of invalid frames and consecutive errors
 * - Support for bytestuffing
 *
 * Usage:
 * - Initialize the parser using parser_init()
 * - Feed bytes using parser_feed_byte()
 * - Check for completed packets and errors
 * - Retrieve the number of consecutive invalid frames
 *
 * @author Nick Constant + ChatGPT
 * @date 2025-11-11
 */

#include "parser.h"
#include <string.h>

#define USE_CRC16 1
#define USE_BYTESTUFF 1

enum
{
    S_WAIT_SYNC = 0,
    S_READ_LEN,
    S_READ_TYPE,
    S_READ_PAYLOAD
};

void parser_init(parser_t *p)
{
    p->state = S_WAIT_SYNC;
    p->len = 0;
    p->typ = 0;
    p->payload_idx = 0;
    p->sum = 0;
    p->invalid_consec = 0;
    memset(p->payload, 0, sizeof(p->payload));
}

unsigned int parser_invalid_consecutive(const parser_t *p)
{
    return p->invalid_consec;
}

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
#endif

/* calculate expected checksum: ~((LEN + TYPE + sum(payload)) & 0xFF) */
static uint8_t calc_checksum(uint8_t len, uint8_t typ, const uint8_t *payload, size_t payload_len)
{
    uint16_t s = len + typ;
    for (size_t i = 0; i < payload_len; ++i)
        s += payload[i];
    return (uint8_t)(~((uint8_t)(s & 0xFF)));
}

int parser_feed_byte(parser_t *p, uint8_t b, packet_t *out)
{
#if USE_BYTESTUFF
    static uint8_t esc_next = 0;
    if (esc_next)
    {
        b ^= 0x20;
        esc_next = 0;
    }
    else if (b == 0x7D)
    {
        esc_next = 1;
        return 0;
    }
#endif

    switch (p->state)
    {
    case S_WAIT_SYNC:
        if (b == SYNC_BYTE)
        {
            p->state = S_READ_LEN;
            return 0;
        }
        else
        {
            /* count noise as invalid attempt (per spec example) */
            p->invalid_consec++;
            return -1;
        }
    case S_READ_LEN:
        p->len = b;
        if (p->len < LEN_MIN || p->len > LEN_MAX)
        {
            p->invalid_consec++;
            p->state = S_WAIT_SYNC;
            p->payload_idx = 0;
            p->sum = 0;
            return -1;
        }
        p->state = S_READ_TYPE;
        return 0;
    case S_READ_TYPE:
        p->typ = b;
        p->payload_idx = 0;
        p->sum = 0;
        /* payload length = LEN - TYPE - CHECKSUM = LEN - 2 */
        {
            uint8_t payload_len = (p->len >= 2) ? (uint8_t)(p->len - 2) : 0;
            if (payload_len > PING_PAYLOAD_MAX)
            {
                /* declared payload longer than allowed -> invalid */
                p->invalid_consec++;
                p->state = S_WAIT_SYNC;
                return -1;
            }
        }
        p->state = S_READ_PAYLOAD;
        return 0;
    case S_READ_PAYLOAD:
    {
        uint8_t payload_len = (p->len >= 2) ? (uint8_t)(p->len - 2) : 0;
        if (p->payload_idx < payload_len)
        {
            /* collecting payload */
            if (p->payload_idx < PING_PAYLOAD_MAX)
            {
                p->payload[p->payload_idx++] = b;
                p->sum = (uint16_t)(p->sum + b);
                return 0;
            }
            else
            {
                /* shouldn't happen because we validated payload_len, but be defensive */
                p->invalid_consec++;
                p->state = S_WAIT_SYNC;
                p->payload_idx = 0;
                p->sum = 0;
                return -1;
            }
        }
        else
        {
/* This byte is checksum */
#if USE_CRC16
            uint8_t expected_low = 0, expected_high = 0;
            uint16_t crc = crc16_ccitt((const uint8_t[]){p->len, p->typ}, 2);
            crc = crc16_ccitt(p->payload, payload_len);
            uint16_t expected_crc = crc; /* demo assumes single byte checksum = LSB */
            if (b != (uint8_t)(expected_crc & 0xFF))
            {
#else
            uint8_t expected = calc_checksum(p->len, p->typ, p->payload, payload_len);
            if (b != expected)
            {
#endif
                p->invalid_consec++;
                p->state = S_WAIT_SYNC;
                p->payload_idx = 0;
                p->sum = 0;
                return -1;
            }
            else
            {
                /* valid frame -> populate out */
                p->invalid_consec = 0;
                if (p->typ == T_START)
                {
                    out->type = PKG_START;
                    out->payload_len = 0;
                }
                else if (p->typ == T_STOP)
                {
                    out->type = PKG_STOP;
                    out->payload_len = 0;
                }
                else if (p->typ == T_PING)
                {
                    out->type = PKG_PING;
                    out->payload_len = payload_len;
                    memcpy(out->payload, p->payload, payload_len);
                }
                else if (p->typ == T_RESET)
                {
                    out->type = PKG_RESET;
                    out->payload_len = 0;
                }
                else
                {
                    out->type = PKG_UNKNOWN;
                    out->payload_len = 0;
                }
                /* reset parser state to look for next SYNC */
                p->state = S_WAIT_SYNC;
                p->payload_idx = 0;
                p->sum = 0;
                return 1;
            }
        }
    }
    default:
        p->state = S_WAIT_SYNC;
        return -1;
    }
}