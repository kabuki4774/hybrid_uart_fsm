/**
 * @file ringbuf.c
 * @brief Implementation of a simple ring buffer.
 * This file contains the implementation of a fixed-size
 * ring buffer (circular buffer) for byte storage.
 *
 * Features:
 * - Fixed capacity defined by RB_CAPACITY
 * - Push and pop operations
 * - Full and empty state checks
 * - Length query
 *
 * Usage:
 * - Initialize the ring buffer using rb_init()
 * - Use rb_push() to add bytes
 * - Use rb_pop() to remove bytes
 * - Check buffer state with rb_is_empty() and rb_is_full()
 * - Get current length with rb_len()
 *
 * @author Nick Constant + ChatGPT
 * @date 2024-06-15
 */

#include "ringbuf.h"

void rb_init(ringbuf_t *rb)
{
    rb->head = rb->tail = 0;
    rb->full = false;
}

static size_t rb_next(size_t i)
{
    return (i + 1) % RB_CAPACITY;
}

bool rb_is_empty(const ringbuf_t *rb)
{
    return (!rb->full) && (rb->head == rb->tail);
}

bool rb_is_full(const ringbuf_t *rb)
{
    return rb->full;
}

size_t rb_len(const ringbuf_t *rb)
{
    if (rb->full)
        return RB_CAPACITY;
    if (rb->head >= rb->tail)
        return rb->head - rb->tail;
    return RB_CAPACITY - (rb->tail - rb->head);
}

int rb_push(ringbuf_t *rb, uint8_t b)
{
    if (rb->full)
        return -1;
    rb->buf[rb->head] = b;
    rb->head = rb_next(rb->head);
    rb->full = (rb->head == rb->tail);
    return 0;
}

int rb_pop(ringbuf_t *rb, uint8_t *out)
{
    if (rb_is_empty(rb))
        return -1;
    *out = rb->buf[rb->tail];
    rb->tail = rb_next(rb->tail);
    rb->full = false;
    return 0;
}