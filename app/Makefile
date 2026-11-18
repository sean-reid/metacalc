SPASM ?= spasm
OUT   := build/metacalc.8xk

$(OUT): src/metacalc.z80 src/equates.inc
	mkdir -p build
	$(SPASM) -I src src/metacalc.z80 $(OUT)

check: $(OUT)
	sh scripts/check-8xk.sh $(OUT)

clean:
	rm -rf build

.PHONY: check clean
