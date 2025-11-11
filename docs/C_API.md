# C API Reference

## Parser

```c
void parser_init(parser_t *p);
int parser_feed_byte(parser_t *p, uint8_t byte, packet_t *out);
unsigned int parser_invalid_consecutive(const parser_t *p);

### FSM
void fsm_init(fsm_t *f);
void fsm_tick(fsm_t *f, uint32_t now_ms);
void fsm_on_invalid_consecutive(fsm_t *f, unsigned int n);
void fsm_handle_packet(fsm_t *f, const packet_t *pkt);

### Harness Helpers
void run_demos(void);
void run_from_stdin(void);
void run_from_serial(const char *device);   // optional


All functions are reentrant and allocate no dynamic memory.