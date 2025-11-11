/**
 * @file fsm.c
 * @brief Implementation of the UART FSM.
 *
 * This file contains the implementation of the finite state machine (FSM)
 * that manages the states and transitions based on incoming packets and
 * timing events.
 *
 * Features:
 * - State transitions: Idle, Active, Error
 * - Heartbeat generation
 * - Handling of valid and invalid packets
 * - Timeout management for inactivity
 * - Configurable invalid packet threshold
 * - Tickless mode support for low-power operation
 *
 * Usage:
 * - Initialize the FSM using fsm_init()
 * - Call fsm_tick() periodically to update the FSM state based on time
 * - Use fsm_handle_packet() to process incoming packets
 * - Optionally, use fsm_next_deadline_ms() in tickless mode to get
 *  the next deadline for processing
 *
 * @author Nick Constant + ChatGPT
 * @date 2025-11-11
 */

#include "fsm.h"
#include <stdio.h>
#include <string.h>

void fsm_init(fsm_t *f)
{
    f->state = ST_IDLE;
    f->now_ms = 0;
    f->active_since_ms = 0;
    f->last_cmd_ms = 0;
    f->next_hb_ms = 0;
    f->invalid_threshold = 3;
}

void fsm_tick(fsm_t *f, uint32_t now_ms)
{
    f->now_ms = now_ms;
    if (f->state == ST_ACTIVE && now_ms >= f->next_hb_ms)
    {
        uint32_t elapsed = now_ms - f->active_since_ms;
        /* print heartbeat rounded down to nearest 1000 */
        printf("HEARTBEAT %u\n", (elapsed / 1000) * 1000);
        f->next_hb_ms += 1000;
    }
    if (f->state == ST_ACTIVE && (now_ms - f->last_cmd_ms) >= 5000)
    {
        printf("STATE: Active -> Idle (inactivity)\n");
        f->state = ST_IDLE;
    }
}

void fsm_on_invalid_consecutive(fsm_t *f, unsigned int n)
{
    if (f->state != ST_ERROR && n >= f->invalid_threshold)
    {
        printf("ERRORS: %u invalid frames -> STATE: * -> Error\n", n);
        f->state = ST_ERROR;
    }
}

void fsm_handle_packet(fsm_t *f, const packet_t *pkt)
{
    switch (pkt->type)
    {
    case PKG_START:
        if (f->state == ST_IDLE)
        {
            printf("STATE: Idle -> Active\n");
            f->state = ST_ACTIVE;
            f->active_since_ms = f->now_ms;
            f->last_cmd_ms = f->now_ms;
            f->next_hb_ms = f->now_ms + 1000;
        }
        else if (f->state == ST_ACTIVE)
        {
            /* refresh inactivity */
            f->last_cmd_ms = f->now_ms;
        }
        break;
    case PKG_STOP:
        if (f->state == ST_ACTIVE)
        {
            printf("STATE: Active -> Idle (STOP)\n");
            f->state = ST_IDLE;
        }
        break;
    case PKG_PING:
        if (f->state != ST_ERROR)
        {
            if (pkt->payload_len == 0)
            {
                printf("PONG\n");
            }
            else
            {
                /* ensure payload printable: assume ASCII */
                char tmp[PING_PAYLOAD_MAX + 1];
                size_t n = pkt->payload_len;
                if (n > PING_PAYLOAD_MAX)
                    n = PING_PAYLOAD_MAX;
                memcpy(tmp, pkt->payload, n);
                tmp[n] = '\0';
                printf("PONG %s\n", tmp);
            }
            f->last_cmd_ms = f->now_ms;
        }
        break;
    case PKG_RESET:
        if (f->state == ST_ERROR)
        {
            printf("STATE: Error -> Idle (RESET)\n");
            f->state = ST_IDLE;
        }
        break;
    default:
        /* unknown type */
        printf("WARN: Unknown packet type\n");
        break;
    }
}

#if USE_TICKLESS
#include <stdint.h>
uint32_t fsm_next_deadline_ms(const fsm_t *f)
{
    if (f->state != ST_ACTIVE)
        return 0;
    uint32_t hb = f->next_hb_ms;
    uint32_t idle = f->last_cmd_ms + 5000;
    return (hb < idle) ? hb : idle;
}
#endif