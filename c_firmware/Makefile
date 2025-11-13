CC ?= clang
RUST_PROFILE ?= release
# Workspace target dir:
RUST_LIB_DIR = ../target/$(RUST_PROFILE)
CFLAGS  = -std=c11 -O2 -Wall -Wextra
LDFLAGS = -L$(RUST_LIB_DIR) -lrust_logic -lpthread -ldl -lm

.PHONY: rustlib
rustlib:
	cargo build --manifest-path ../rust_logic/Cargo.toml --lib \
	  --no-default-features --features "$(RUST_FEATURES)" \
	  $(if $(filter $(RUST_PROFILE),release),--release,)

all: rustlib main

main: main.c
	$(CC) $(CFLAGS) -o $@ $< $(LDFLAGS)

run: main
	./main

clean:
	rm -f main
	cargo clean -p rust_logic