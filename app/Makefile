SPASM   ?= spasm
BUILD   ?= build
LIBRARY ?=
# The stage table, scripts/stages-include.sh's input: stages.list beside LIBRARY unless one is given.
# A build with no list at that default has one stage, the whole library.
STAGES  ?= $(shell dirname "$(LIBRARY)")/stages.list
# A directory holding font.inc; by default the one mcglyph writes into BUILD.
FONT    ?=
OUT     := $(BUILD)/metacalc.8xk
ASM     := SPASM="$(SPASM)" sh ../scripts/spasm.sh
INCLUDE := -I ../kernel -I src -I .. -I ../ui
# The workspace version the host model reports as CARGO_PKG_VERSION, as the numbers About shows:
# the line `version = "major.minor.patch"` of [workspace.package] and nothing else on it. A line of
# another form reads as `invalid`; the app target refuses that, so `make clean` works with any.
WORKSPACE_TOML := ../Cargo.toml
VERSION := $(shell awk '/^\[/ { in_package = ($$0 == "[workspace.package]") } in_package && /^version *=/ { print (($$0 ~ /^version = "[0-9]+\.[0-9]+\.[0-9]+"$$/) ? substr($$0, 12, length($$0) - 12) : "invalid"); exit }' $(WORKSPACE_TOML))
VERSION_PARTS := $(subst ., ,$(VERSION))
INCLUDE += -DVERSION_MAJOR=$(word 1,$(VERSION_PARTS)) -DVERSION_MINOR=$(word 2,$(VERSION_PARTS)) -DVERSION_PATCH=$(word 3,$(VERSION_PARTS))

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
	@printf '%s\n' '$(VERSION)' | grep -Eq '^[0-9]+\.[0-9]+\.[0-9]+$$' || { echo 'app: the version in $(WORKSPACE_TOML) is not a line version = "major.minor.patch": "$(VERSION)"' >&2; exit 1; }
	mkdir -p "$(BUILD)"
ifneq ($(FONT),)
	@test -f "$(FONT)/font.inc" || { echo "app: $(FONT)/font.inc is missing" >&2; exit 1; }
endif
ifeq ($(FONT),)
	cargo run --quiet --locked --manifest-path ../Cargo.toml -p mcglyph -- z80 "$(BUILD)/font.inc" > /dev/null
endif
ifneq ($(LIBRARY),)
	sh scripts/library-include.sh "$(LIBRARY)" > "$(BUILD)/library.inc"
ifeq ($(origin STAGES),file)
	list="$(STAGES)"; [ -f "$$list" ] || list=; sh scripts/stages-include.sh "$(LIBRARY)" "$$list" > "$(BUILD)/stages.inc"
else
	sh scripts/stages-include.sh "$(LIBRARY)" "$(STAGES)" > "$(BUILD)/stages.inc"
endif
endif
	$(ASM) -L $(FLAGS) src/metacalc.z80 "$(OUT)"

check: app
	$(CHECK)
	$(SELF)
	sh scripts/test-library-pages.sh "$(SPASM)"
	sh scripts/test-stages.sh "$(SPASM)"

clean:
	rm -rf "$(BUILD)"

.PHONY: app check clean
