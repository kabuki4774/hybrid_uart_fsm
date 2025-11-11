#ifndef RINGBUF_H
#define RINGBUF_H

#include <stddef.h>
#include <stdint.h>
#include <stdbool.h>

#define RB_CAPACITY 128

typedef struct
{
    uint8_t buf[RB_CAPACITY];
    size_t head;
    size_t tail;
    bool full;
} ringbuf_t;

void rb_init(ringbuf_t *rb);
bool rb_is_empty(const ringbuf_t *rb);
bool rb_is_full(const ringbuf_t *rb);
size_t rb_len(const ringbuf_t *rb);
int rb_push(ringbuf_t *rb, uint8_t b);   /* returns 0 on success, -1 on overflow */
int rb_pop(ringbuf_t *rb, uint8_t *out); /* returns 0 on success, -1 when empty */

#endif /* RINGBUF_H */