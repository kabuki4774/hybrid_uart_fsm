#ifndef FSM_H
#define FSM_H

#include <stdint.h>
#include "parser.h"

#ifndef USE_TICKLESS
#define USE_TICKLESS 1
#endif

typedef enum
{
    ST_IDLE = 0,
    ST_ACTIVE,
    ST_ERROR
} fsm_state_t;

typedef struct
{
    fsm_state_t state;
    uint32_t now_ms;
    uint32_t active_since_ms;
    uint32_t last_cmd_ms;
    uint32_t next_hb_ms;
    unsigned int invalid_threshold;
} fsm_t;

void fsm_init(fsm_t *f);
void fsm_tick(fsm_t *f, uint32_t now_ms);
void fsm_on_invalid_consecutive(fsm_t *f, unsigned int n);
void fsm_handle_packet(fsm_t *f, const packet_t *pkt);
#if USE_TICKLESS
uint32_t fsm_next_deadline_ms(const fsm_t *f);
#endif

#endif /* FSM_H */