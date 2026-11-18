SPASM ?= spasm

all: build/ramprobe.8xk build/sparecheck.8xk

build/ramprobe.8xk: ramprobe.z80 ../app/src/equates.inc
	mkdir -p build
	$(SPASM) -I ../app/src ramprobe.z80 $@

build/sparecheck.8xk: sparecheck.z80 sparecore.inc ../app/src/equates.inc
	mkdir -p build
	$(SPASM) -I ../app/src -I . sparecheck.z80 $@

check: all
	sh ../app/scripts/check-8xk.sh build/ramprobe.8xk ramprobe
	sh ../app/scripts/test-check-8xk.sh build/ramprobe.8xk ramprobe
	sh ../app/scripts/check-8xk.sh build/sparecheck.8xk sparechk
	sh ../app/scripts/test-check-8xk.sh build/sparecheck.8xk sparechk

clean:
	rm -rf build

.PHONY: all check clean
