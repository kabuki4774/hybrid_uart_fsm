CC ?= clang
CFLAGS_BASE = -std=c11 -O2 -Wall -Wextra -I./src

# --- Feature flags (defaults) -----------------------------------------
crc16 ?= 0
byte_stuff ?= 0
tickless ?= 0
serial ?= 0

CFLAGS += $(CFLAGS_BASE)
CFLAGS += -DUSE_CRC16=$(crc16) -DUSE_BYTESTUFF=$(byte_stuff) \
          -DUSE_TICKLESS=$(tickless) -DUSE_SERIAL=$(serial)

SRCS = src/main.c src/ringbuf.c src/parser.c src/fsm.c src/harness.c
OBJS = $(SRCS:.c=.o)
BIN = uart_fsm_demo

.PHONY: all clean run test

all: $(BIN)

$(BIN): $(OBJS)
	$(CC) $(CFLAGS) -o $@ $^

%.o: %.c
	$(CC) $(CFLAGS) -c -o $@ $<

run: $(BIN)
	./$(BIN)

test: $(BIN)
	./$(BIN) --test

clean:
	rm -f $(OBJS) $(BIN)

lib:
	$(CC) $(CFLAGS) -c src/ringbuf.c src/parser.c src/fsm.c src/harness.c
	ar rcs libuartfsm.a ringbuf.o parser.o fsm.o harness.o