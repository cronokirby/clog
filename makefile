# Source
SRC      := src/main.c

# Output binaries
DEBUG_BIN   := target/debug
RELEASE_BIN := target/release

# Common flags (always applied)
COMMON_CFLAGS := -std=c23 \
                 -Wall -Wextra -Wpedantic -Wshadow \
                 -Wconversion -Wuninitialized -Wundef \
                 -Werror=implicit-function-declaration

# Debug-only flags
DEBUG_CFLAGS := -fsanitize=address,undefined -g

# Release-only flags
# Do NOT specify -O2 or -O3; Nix stdenv already injects optimization flags.
RELEASE_CFLAGS :=

.PHONY: debug release clean

debug: $(DEBUG_BIN)

$(DEBUG_BIN): $(SRC)
	mkdir -p target
	$(CC) $(COMMON_CFLAGS) $(DEBUG_CFLAGS) $< -o $@

release: $(RELEASE_BIN)

$(RELEASE_BIN): $(SRC)
	mkdir -p target
	$(CC) $(COMMON_CFLAGS) $(RELEASE_CFLAGS) $< -o $@

clean:
	rm -rf target
