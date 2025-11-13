CC ?= clang
CFLAGS_BASE = -std=c11 -O2 -Wall -Wextra -I./src

# --- Feature flags (override at command line: `make crc16=0` etc.) -----
crc16     ?= 1          # default on (matches Rust default when enabled)
byte_stuff?= 1
tickless  ?= 1
serial    ?= 1

CFLAGS += $(CFLAGS_BASE)
CFLAGS += -DUSE_CRC16=$(crc16) -DUSE_BYTESTUFF=$(byte_stuff) \
          -DUSE_TICKLESS=$(tickless) -DUSE_SERIAL=$(serial)

SRCS = src/main.c src/ringbuf.c src/parser.c src/fsm.c src/harness.c
OBJS = $(SRCS:.c=.o)
BIN = uart_fsm_demo

.PHONY: all clean run test lib

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
	rm -f $(OBJS) $(BIN) libuartfsm.a ringbuf.o parser.o fsm.o harness.o

lib:
	$(CC) $(CFLAGS) -c src/ringbuf.c src/parser.c src/fsm.c src/harness.c
	ar rcs libuartfsm.a ringbuf.o parser.o fsm.o harness.o