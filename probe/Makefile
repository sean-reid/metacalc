SPASM   ?= spasm
BUILD   ?= build
LIBRARY ?=

all: $(BUILD)/ramprobe.8xk $(BUILD)/sparecheck.8xk $(BUILD)/storecheck.8xk
ifneq ($(LIBRARY),)
all: kernchk
endif

$(BUILD)/ramprobe.8xk: ramprobe.z80 ../app/src/equates.inc
	mkdir -p "$(BUILD)"
	"$(SPASM)" -I ../app/src ramprobe.z80 $@

$(BUILD)/sparecheck.8xk: sparecheck.z80 sparecore.inc sparescreen.inc printnum.inc ../app/src/equates.inc
	mkdir -p "$(BUILD)"
	"$(SPASM)" -I ../app/src -I . sparecheck.z80 $@

$(BUILD)/storecheck.8xk: storecheck.z80 ../app/src/store.inc ../app/src/header.inc ../app/src/equates.inc
	mkdir -p "$(BUILD)"
	"$(SPASM)" -I ../app/src storecheck.z80 $@

# A phony target, so a path with a space is never a make target, and the app is always assembled
# because make cannot see LIBRARY change. It needs a library, so it is only built when one is given.
kernchk:
	mkdir -p "$(BUILD)"
	sh ../app/scripts/library-include.sh "$(LIBRARY)" > "$(BUILD)/library.inc"
	"$(SPASM)" -L -I ../app/src -I ../kernel -I . -I "$(BUILD)" kerncheck.z80 "$(BUILD)/kernchk.8xk"

check: all
	sh ../app/scripts/check-8xk.sh $(BUILD)/ramprobe.8xk ramprobe
	sh ../app/scripts/test-check-8xk.sh $(BUILD)/ramprobe.8xk ramprobe
	sh ../app/scripts/check-8xk.sh $(BUILD)/sparecheck.8xk sparechk
	sh ../app/scripts/test-check-8xk.sh $(BUILD)/sparecheck.8xk sparechk
	sh ../app/scripts/check-8xk.sh $(BUILD)/storecheck.8xk storechk
	sh ../app/scripts/test-check-8xk.sh $(BUILD)/storecheck.8xk storechk
ifneq ($(LIBRARY),)
	sh ../app/scripts/check-8xk.sh -l "$(LIBRARY)" "$(BUILD)/kernchk.8xk" kernchk
	sh ../app/scripts/test-check-8xk.sh -l "$(LIBRARY)" "$(BUILD)/kernchk.8xk" kernchk
endif

clean:
	rm -rf "$(BUILD)"

.PHONY: all check clean kernchk
