SPASM   ?= spasm
BUILD   ?= build
LIBRARY ?=
# A directory holding font.inc; by default the one mcglyph writes into BUILD.
FONT    ?=
OUT     := $(BUILD)/metacalc.8xk
ASM     := SPASM="$(SPASM)" sh ../scripts/spasm.sh
INCLUDE := -I ../kernel -I src -I .. -I ../ui

ifeq ($(FONT),)
FONTDIR := $(BUILD)
else
FONTDIR := $(FONT)
endif

ifeq ($(LIBRARY),)
FLAGS := $(INCLUDE) -I "$(FONTDIR)"
CHECK := sh scripts/check-8xk.sh -p 3 "$(OUT)"
SELF  := sh scripts/test-check-8xk.sh -p 3 "$(OUT)"
else
FLAGS := $(INCLUDE) -I "$(FONTDIR)" -I "$(BUILD)" -DLIBRARY
CHECK := sh scripts/check-8xk.sh -p 3 -l "$(LIBRARY)" "$(OUT)"
SELF  := sh scripts/test-check-8xk.sh -p 3 -l "$(LIBRARY)" "$(OUT)"
endif

# A phony target, so a path with a space is never a make target, and the app is always
# assembled because make cannot see LIBRARY change.
app:
	mkdir -p "$(BUILD)"
ifneq ($(FONT),)
	@test -f "$(FONT)/font.inc" || { echo "app: $(FONT)/font.inc is missing" >&2; exit 1; }
endif
ifeq ($(FONT),)
	cargo run --quiet --locked --manifest-path ../Cargo.toml -p mcglyph -- z80 "$(BUILD)/font.inc" > /dev/null
endif
ifneq ($(LIBRARY),)
	sh scripts/library-include.sh "$(LIBRARY)" > "$(BUILD)/library.inc"
endif
	$(ASM) -L $(FLAGS) src/metacalc.z80 "$(OUT)"

check: app
	$(CHECK)
	$(SELF)
	sh scripts/test-library-pages.sh "$(SPASM)"

clean:
	rm -rf "$(BUILD)"

.PHONY: app check clean
