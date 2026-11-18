SPASM   ?= spasm
BUILD   ?= build
LIBRARY ?=
OUT     := $(BUILD)/metacalc.8xk

ifeq ($(LIBRARY),)
FLAGS := -I src
CHECK := sh scripts/check-8xk.sh "$(OUT)"
SELF  := sh scripts/test-check-8xk.sh "$(OUT)"
else
FLAGS := -I src -I "$(BUILD)" -DLIBRARY
CHECK := sh scripts/check-8xk.sh -l "$(LIBRARY)" "$(OUT)"
SELF  := sh scripts/test-check-8xk.sh -l "$(LIBRARY)" "$(OUT)"
endif

# A phony target, so a path with a space is never a make target, and the app is always
# assembled because make cannot see LIBRARY change.
app:
	mkdir -p "$(BUILD)"
ifneq ($(LIBRARY),)
	sh scripts/library-include.sh "$(LIBRARY)" > "$(BUILD)/library.inc"
endif
	"$(SPASM)" -L $(FLAGS) src/metacalc.z80 "$(OUT)"

check: app
	$(CHECK)
	$(SELF)
	sh scripts/test-library-pages.sh "$(SPASM)"

clean:
	rm -rf "$(BUILD)"

.PHONY: app check clean
