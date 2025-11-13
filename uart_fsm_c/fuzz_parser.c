#include "src/parser.h"
#include <stdint.h>
#include <stddef.h>

int LLVMFuzzerTestOneInput(const uint8_t *data, size_t size)
{
    parser_t p;
    parser_init(&p);
    packet_t pkt;
    for (size_t i = 0; i < size; i++)
    {
        parser_feed_byte(&p, data[i], &pkt);
    }
    return 0;
}