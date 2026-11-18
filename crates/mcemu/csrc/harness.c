#include <stdarg.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <pthread.h>

#include "scancodes.h"
#include "tilem.h"
#include "xz/xz.h"

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
/* The core reports an opcode as prefix and byte (DDCB0076 for bit 6,(ix+0)) and DD 76 and FD 76 as 76; the whole word must match. */
#define OP_HALT 0x76
#define OP_MASK 0xffffffffu
#define STUB_END 0x4000

/* A HALT ends a run unless interrupts are on and it is not the stub's own: the app wakes from it. */
static int halt_stops(TilemCalc *calc, dword op, void *data)
{
	(void)op;
	(void)data;
	return !calc->z80.r.iff1 || (uint16_t)(calc->z80.r.pc.w.l - 1) < STUB_END;
}

/* The calculator has one spare RAM page for 82h to 87h; the core models six. */
#define SPARE_FIRST 0x82
#define SPARE_LAST 0x87

static void alias_spare_pages(TilemCalc *calc)
{
	int i;

	for (i = 1; i < 4; i++)
		if (calc->mempagemap[i] > SPARE_FIRST && calc->mempagemap[i] <= SPARE_LAST)
			calc->mempagemap[i] = SPARE_FIRST;
}

/* The core holds the LCD busy for a fixed count of CPU clocks, 50 at 6 MHz. The controller takes a fixed time, so the count scales with the clock. */
#define LCD_BUSY_BASE_KHZ 6000

static dword lcd_busy_expiry(TilemCalc *calc)
{
	if (!calc->lcd.busy)
		return 0;
	return calc->z80.clock + tilem_z80_get_timer_clocks(calc, TILEM_TIMER_LCD_DELAY);
}

static void scale_lcd_busy(TilemCalc *calc, dword before)
{
	dword after = lcd_busy_expiry(calc);

	if (after == before || calc->z80.clockspeed == LCD_BUSY_BASE_KHZ)
		return;
	tilem_z80_set_timer(calc, TILEM_TIMER_LCD_DELAY,
			    (after - calc->z80.clock) * calc->z80.clockspeed / LCD_BUSY_BASE_KHZ, 0, 0);
}

static int is_lcd_port(dword port)
{
	return (port & 0xff & 0xfc) == 0x10;
}

/* What the harness keeps for each machine: the set of values written to ports 5, 6 and 7 since its
   last run began, and whether the LCD busy bit is forced on. Found by the core's pointer. */
typedef struct {
	TilemCalc *calc;
	uint8_t banks[3][32];
	int stuck;
} McState;

#define MAX_MACHINES 512
static McState states[MAX_MACHINES];
static pthread_mutex_t states_lock = PTHREAD_MUTEX_INITIALIZER;
static _Thread_local McState *last_state;

static McState *state_of(TilemCalc *calc)
{
	int i;

	if (last_state && last_state->calc == calc)
		return last_state;
	pthread_mutex_lock(&states_lock);
	for (i = 0; i < MAX_MACHINES; i++)
		if (states[i].calc == calc) {
			last_state = &states[i];
			break;
		}
	pthread_mutex_unlock(&states_lock);
	return i < MAX_MACHINES ? &states[i] : NULL;
}

static void claim_state(TilemCalc *calc)
{
	int i;

	pthread_mutex_lock(&states_lock);
	for (i = 0; i < MAX_MACHINES && states[i].calc; i++)
		;
	if (i == MAX_MACHINES)
		abort();
	memset(&states[i], 0, sizeof states[i]);
	states[i].calc = calc;
	pthread_mutex_unlock(&states_lock);
}

static void release_state(TilemCalc *calc)
{
	McState *st = state_of(calc);

	if (st) {
		pthread_mutex_lock(&states_lock);
		st->calc = NULL;
		pthread_mutex_unlock(&states_lock);
	}
	last_state = NULL;
}

static void note_bank_write(TilemCalc *calc, dword port, byte value)
{
	McState *st;

	port &= 0xff;
	if (port >= 5 && port <= 7 && (st = state_of(calc)))
		st->banks[port - 5][value >> 3] |= 1 << (value & 7);
}

static void out_aliased(TilemCalc *calc, dword port, byte value)
{
	dword before = lcd_busy_expiry(calc);

	note_bank_write(calc, port, value);

	xz_z80_out(calc, port, value);
	alias_spare_pages(calc);
	if (is_lcd_port(port))
		scale_lcd_busy(calc, before);
}

static byte in_scaled(TilemCalc *calc, dword port)
{
	dword before = lcd_busy_expiry(calc);
	byte value = xz_z80_in(calc, port);
	McState *st;

	if (is_lcd_port(port))
		scale_lcd_busy(calc, before);
	if ((port & 0xfd) == 0x10 && (st = state_of(calc)) && st->stuck)
		value |= 0x80;
	return value;
}

typedef struct {
	uint16_t af, bc, de, hl, ix, iy, sp, pc, af2, bc2, de2, hl2;
	uint8_t i, iff1, iff2, im;
} McRegs;

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
	claim_state(calc);
	calc->hw.z80_out = out_aliased;
	calc->hw.z80_in = in_scaled;
	alias_spare_pages(calc);
	calc->z80.emuflags |= TILEM_Z80_BREAK_EXCEPTIONS;
	tilem_z80_add_breakpoint(calc, TILEM_BREAK_EXECUTE, OP_HALT, OP_HALT, OP_MASK, halt_stops, NULL);
	return calc;
}

void mcemu_free(TilemCalc *calc)
{
	release_state(calc);
	tilem_calc_free(calc);
}
uint8_t *mcemu_mem(TilemCalc *calc) { return calc->mem; }
size_t mcemu_rom_size(const TilemCalc *calc) { return calc->hw.romsize; }
size_t mcemu_ram_size(const TilemCalc *calc) { return calc->hw.ramsize; }
uint32_t mcemu_ltop(TilemCalc *calc, uint32_t addr) { return calc->hw.mem_ltop(calc, addr); }
void mcemu_out(TilemCalc *calc, uint8_t port, uint8_t value) { calc->hw.z80_out(calc, port, value); }
uint8_t mcemu_in(TilemCalc *calc, uint8_t port) { return calc->hw.z80_in(calc, port); }
int mcemu_khz(const TilemCalc *calc) { return calc->z80.clockspeed; }
/* Writes a port with the flash protection open, as the OS does for the protected ones. */
void mcemu_out_unlocked(TilemCalc *calc, uint8_t port, uint8_t value)
{
	byte was = calc->flash.unlock;

	calc->flash.unlock = 1;
	calc->hw.z80_out(calc, port, value);
	calc->flash.unlock = was;
}

void mcemu_get_regs(const TilemCalc *calc, McRegs *out)
{
	const TilemZ80Regs *r = &calc->z80.r;

	out->af = r->af.w.l; out->bc = r->bc.w.l; out->de = r->de.w.l; out->hl = r->hl.w.l;
	out->ix = r->ix.w.l; out->iy = r->iy.w.l; out->sp = r->sp.w.l; out->pc = r->pc.w.l;
	out->af2 = r->af2.w.l; out->bc2 = r->bc2.w.l; out->de2 = r->de2.w.l; out->hl2 = r->hl2.w.l;
	out->i = r->ir.b.h; out->iff1 = r->iff1; out->iff2 = r->iff2; out->im = r->im;
}

void mcemu_set_regs(TilemCalc *calc, const McRegs *in)
{
	TilemZ80Regs *r = &calc->z80.r;

	r->af.w.l = in->af; r->bc.w.l = in->bc; r->de.w.l = in->de; r->hl.w.l = in->hl;
	r->ix.w.l = in->ix; r->iy.w.l = in->iy; r->sp.w.l = in->sp; r->pc.w.l = in->pc;
	r->af2.w.l = in->af2; r->bc2.w.l = in->bc2; r->de2.w.l = in->de2; r->hl2.w.l = in->hl2;
	r->ir.b.h = in->i; r->iff1 = in->iff1; r->iff2 = in->iff2; r->im = in->im;
	calc->z80.halted = 0;
}

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

	memset(state_of(calc)->banks, 0, sizeof state_of(calc)->banks);
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

/* Runs on from the current state, interrupts as the program left them. */
uint32_t mcemu_resume(TilemCalc *calc, int limit, uint32_t *elapsed)
{
	uint32_t start = calc->z80.clock;
	uint32_t reason;

	memset(state_of(calc)->banks, 0, sizeof state_of(calc)->banks);
	reason = tilem_z80_run(calc, limit, NULL);
	*elapsed = calc->z80.clock - start;
	return reason;
}

/* Copies the bit sets of values written to ports 5, 6 and 7 since this machine's last run or resume began. */
void mcemu_banks(TilemCalc *calc, uint8_t out[3][32]) { memcpy(out, state_of(calc)->banks, sizeof state_of(calc)->banks); }

/* Forces bit 7 of the LCD status port on, as a controller that never finishes would. */
void mcemu_stick_lcd_busy(TilemCalc *calc, int on) { state_of(calc)->stuck = on; }

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

/* The controller's memory as one byte per pixel, whatever the LCD-on flag and the halt state say; the row shift applies.
   get_lcd writes lcdwidth/8 bytes for each of lcdheight rows; a canary after them catches an overrun.
   Returns 0 on success, 1 when the size is not the core's, 2 when the core wrote past its rows. */
int mcemu_lcd_ram(TilemCalc *calc, uint8_t *out, int width, int height)
{
	int stride = calc->hw.lcdwidth / 8;
	size_t rows = (size_t)stride * calc->hw.lcdheight;
	uint8_t *bits;
	int x, y, i, overrun = 0;

	if (width != calc->hw.lcdwidth || height != calc->hw.lcdheight)
		return 1;
	bits = malloc(rows + SLACK);
	if (!bits)
		abort();
	memset(bits + rows, 0xA5, SLACK);
	calc->hw.get_lcd(calc, bits);
	for (i = 0; i < SLACK; i++)
		overrun |= bits[rows + i] != 0xA5;
	for (y = 0; !overrun && y < height; y++)
		for (x = 0; x < width; x++)
			out[y * width + x] = bits[y * stride + x / 8] >> (7 - x % 8) & 1;
	free(bits);
	return overrun ? 2 : 0;
}
