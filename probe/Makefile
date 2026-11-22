SPASM   ?= spasm
BUILD   ?= build
LIBRARY ?=
ASM     := SPASM="$(SPASM)" sh ../scripts/spasm.sh

all: $(BUILD)/ramprobe.8xk $(BUILD)/sparecheck.8xk $(BUILD)/storecheck.8xk
ifneq ($(LIBRARY),)
all: kernchk
endif

$(BUILD)/ramprobe.8xk: ramprobe.z80 ramcheck.inc ../app/src/equates.inc ../app/src/ram_map.inc
	mkdir -p "$(BUILD)"
	$(ASM) -I ../app/src ramprobe.z80 $@

$(BUILD)/sparecheck.8xk: sparecheck.z80 ramcheck.inc sparecore.inc sparescreen.inc printnum.inc ../app/src/equates.inc ../app/src/ram_map.inc
	mkdir -p "$(BUILD)"
	$(ASM) -I ../app/src -I . sparecheck.z80 $@

$(BUILD)/storecheck.8xk: storecheck.z80 storescript.inc storeram.inc storeguard.inc ramcheck.inc storedraw.inc ../app/src/store.inc ../app/src/store_count.inc ../app/src/store_open.inc ../app/src/header.inc ../app/src/equates.inc ../app/src/ram_map.inc
	mkdir -p "$(BUILD)"
	"$(SPASM)" -I ../app/src -I . storecheck.z80 $@

# A phony target, so a path with a space is never a make target, and the app is always assembled
# because make cannot see LIBRARY change. It needs a library, so it is only built when one is given.
kernchk:
	mkdir -p "$(BUILD)"
	sh ../app/scripts/library-include.sh "$(LIBRARY)" > "$(BUILD)/library.inc"
	$(ASM) -L -I ../app/src -I ../kernel -I . -I "$(BUILD)" kerncheck.z80 "$(BUILD)/kernchk.8xk"

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

all: $(BUILD)/pagecheck.8xk

$(BUILD)/pagecheck.8xk: pagecheck.z80 ramcheck.inc printnum.inc ../app/src/farcall.inc ../app/src/header.inc ../app/src/equates.inc ../app/src/ram_map.inc
	mkdir -p "$(BUILD)"
	$(ASM) -L -I ../app/src -I . pagecheck.z80 $@

check: check-pagecheck

check-pagecheck: $(BUILD)/pagecheck.8xk
	sh ../app/scripts/check-8xk.sh -p 3 $(BUILD)/pagecheck.8xk pageschk
	sh ../app/scripts/test-check-8xk.sh -p 3 $(BUILD)/pagecheck.8xk pageschk

.PHONY: check-pagecheck

all: $(BUILD)/statecheck.8xk

$(BUILD)/statecheck.8xk: statecheck.z80 statecheck.inc statecompare.inc onbreak.inc printnum.inc ../app/src/header.inc ../app/src/equates.inc ../app/src/ram_map.inc
	mkdir -p "$(BUILD)"
	$(ASM) -L -I ../app/src -I . statecheck.z80 $@

check: check-statecheck

check-statecheck: $(BUILD)/statecheck.8xk
	sh ../app/scripts/check-8xk.sh $(BUILD)/statecheck.8xk statechk
	sh ../app/scripts/test-check-8xk.sh $(BUILD)/statecheck.8xk statechk

.PHONY: check-statecheck
