SPASM   ?= spasm
BUILD   ?= build
LIBRARY ?=
OUT     := $(BUILD)/metacalc.8xk

ifeq ($(LIBRARY),)
FLAGS := -I src
CHECK := sh scripts/check-8xk.sh $(OUT)
else
FLAGS := -I src -I $(BUILD) -DLIBRARY
CHECK := sh scripts/check-8xk.sh -l $(LIBRARY) $(OUT)
endif

# Always assembled: the output depends on LIBRARY, which make cannot see change.
$(OUT): FORCE
	mkdir -p $(BUILD)
ifneq ($(LIBRARY),)
	sh scripts/library-include.sh $(abspath $(LIBRARY)) > $(BUILD)/library.inc
endif
	$(SPASM) -L $(FLAGS) src/metacalc.z80 $(OUT)

check: $(OUT)
	$(CHECK)
	sh scripts/test-check-8xk.sh $(OUT)
	sh scripts/test-library-pages.sh $(SPASM)

clean:
	rm -rf build

FORCE:

.PHONY: check clean FORCE
