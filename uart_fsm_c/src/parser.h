#ifndef PARSER_H
#define PARSER_H

#include <stdint.h>
#include <stddef.h>

/* Packet TYPE codes */
#define T_START 0x01
#define T_STOP 0x02
#define T_PING 0x03
#define T_RESET 0xFF

/* Parser constraints */
#define SYNC_BYTE 0xAA
#define LEN_MIN 2
#define LEN_MAX 32
#define PING_PAYLOAD_MAX 24

typedef enum
{
    PKG_NONE = 0,
    PKG_START,
    PKG_STOP,
    PKG_PING,
    PKG_RESET,
    PKG_UNKNOWN
} pkg_type_t;

typedef struct
{
    pkg_type_t type;
    uint8_t payload[PING_PAYLOAD_MAX];
    size_t payload_len;
} packet_t;

typedef struct
{
    /* internal state, opaque */
    int state;
    uint8_t len;
    uint8_t typ;
    uint8_t payload[PING_PAYLOAD_MAX];
    size_t payload_idx;
    uint16_t sum;
    unsigned int invalid_consec;
} parser_t;

void parser_init(parser_t *p);
unsigned int parser_invalid_consecutive(const parser_t *p);

/* Feed a byte; returns:
   0 -> no complete packet yet
   1 -> a valid packet was produced and written to out
   -1 -> parser resynced due to bad frame (invalid detected)
   Note: consecutive invalids are tracked internally.
*/
int parser_feed_byte(parser_t *p, uint8_t b, packet_t *out);

#endif /* PARSER_H */