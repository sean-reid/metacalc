#include <stdarg.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "scancodes.h"
#include "tilem.h"

void *tilem_malloc(size_t s) { void *p = malloc(s); if (!p) abort(); return p; }
void *tilem_malloc0(size_t s) { void *p = calloc(1, s); if (!p) abort(); return p; }
/* The core's LCD read can run one byte past the memory block it allocates here. */
#define SLACK 16
void *tilem_malloc_atomic(size_t s) { return tilem_malloc0(s + SLACK); }
void *tilem_try_malloc(size_t s) { return malloc(s); }
void *tilem_try_malloc0(size_t s) { return calloc(1, s); }
void *tilem_try_malloc_atomic(size_t s) { return calloc(1, s + SLACK); }
void *tilem_realloc(void *p, size_t s) { return realloc(p, s); }
void tilem_free(void *p) { free(p); }
const char *tilem_gettext(const char *s) { return s; }

void tilem_message(TilemCalc *c, const char *m, ...) { (void)c; (void)m; }

void tilem_warning(TilemCalc *c, const char *m, ...)
{
	va_list a;
	(void)c;
	va_start(a, m);
	fputs("tilem: ", stderr);
	vfprintf(stderr, m, a);
	fputc('\n', stderr);
	va_end(a);
}

void tilem_internal(TilemCalc *c, const char *m, ...)
{
	va_list a;
	(void)c;
	va_start(a, m);
	fputs("tilem internal: ", stderr);
	vfprintf(stderr, m, a);
	fputc('\n', stderr);
	va_end(a);
}

#define ENTRY 0x4000
#define BOOT_LIMIT 1000000
#define OP_HALT 0x76

TilemCalc *mcemu_new(const uint8_t *rom, size_t len)
{
	TilemCalc *calc = tilem_calc_new(TILEM_CALC_TI84P_SE);
	FILE *f = fmemopen((void *)rom, len, "rb");
	int ok = calc && f && tilem_calc_load_state(calc, f, NULL) == 0;
	int bp;

	if (f)
		fclose(f);
	if (!ok) {
		if (calc)
			tilem_calc_free(calc);
		return NULL;
	}
	tilem_calc_reset(calc);
	bp = tilem_z80_add_breakpoint(calc, TILEM_BREAK_MEM_EXEC, ENTRY, ENTRY, 0xffff, NULL, NULL);
	tilem_z80_run(calc, BOOT_LIMIT, NULL);
	tilem_z80_remove_breakpoint(calc, bp);
	if (calc->z80.r.pc.w.l != ENTRY) {
		tilem_calc_free(calc);
		return NULL;
	}
	tilem_z80_add_breakpoint(calc, TILEM_BREAK_EXECUTE, OP_HALT, OP_HALT, 0xffff, NULL, NULL);
	return calc;
}

void mcemu_free(TilemCalc *calc) { tilem_calc_free(calc); }
uint8_t *mcemu_mem(TilemCalc *calc) { return calc->mem; }
size_t mcemu_rom_size(const TilemCalc *calc) { return calc->hw.romsize; }
size_t mcemu_ram_size(const TilemCalc *calc) { return calc->hw.ramsize; }
uint32_t mcemu_ltop(TilemCalc *calc, uint32_t addr) { return calc->hw.mem_ltop(calc, addr); }
void mcemu_out(TilemCalc *calc, uint8_t port, uint8_t value) { calc->hw.z80_out(calc, port, value); }
void mcemu_key(TilemCalc *calc, int scancode, int down)
{
	if (down)
		tilem_keypad_press_key(calc, scancode);
	else
		tilem_keypad_release_key(calc, scancode);
}

/* Returns the stop reason; the T-states elapsed go to *elapsed. */
uint32_t mcemu_run(TilemCalc *calc, uint16_t pc, int limit, uint32_t *elapsed)
{
	uint32_t start;
	uint32_t reason;

	calc->z80.r.iff1 = calc->z80.r.iff2 = 0;
	calc->z80.halted = 0;
	calc->z80.r.pc.d = pc;
	start = calc->z80.clock;
	reason = tilem_z80_run(calc, limit, NULL);
	*elapsed = calc->z80.clock - start;
	return reason;
}

uint16_t mcemu_pc(const TilemCalc *calc)
{
	return calc->z80.r.pc.w.l;
}

/* One byte per pixel, 1 when lit; the core leaves the frame untouched while the LCD is off, so it starts zeroed. Returns 0 on success. */
int mcemu_lcd(TilemCalc *calc, uint8_t *out, int width, int height)
{
	TilemLCDBuffer *buf = tilem_lcd_buffer_new();
	int x, y, bad;

	tilem_lcd_get_frame1(calc, buf);
	bad = buf->width != width || buf->height != height;
	for (y = 0; !bad && y < height; y++)
		for (x = 0; x < width; x++)
			out[y * width + x] = buf->data[y * buf->rowstride + x] ? 1 : 0;
	tilem_lcd_buffer_free(buf);
	return bad;
}
