SPASM ?= spasm
OUT   := build/ramprobe.8xk

$(OUT): ramprobe.z80 ../app/src/equates.inc
	mkdir -p build
	$(SPASM) -I ../app/src ramprobe.z80 $(OUT)

check: $(OUT)
	sh ../app/scripts/check-8xk.sh $(OUT) ramprobe
	sh ../app/scripts/test-check-8xk.sh $(OUT) ramprobe

clean:
	rm -rf build

.PHONY: check clean
