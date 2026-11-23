#include <stdlib.h>
#include <string.h>

#include "watch.h"

#define MAX_RECORDS 65536

struct McWatch {
	int write_bp;
	int exec_bp;
	uint16_t pc;
	McWrite *rec;
	size_t len;
	size_t lost;
};

/* Runs before every instruction, so `pc` is the address of the one whose writes follow. The core
   also runs it for the byte after a prefix, which is part of the same instruction. */
static int note_pc(TilemCalc *calc, dword addr, void *data)
{
	McWatch *w = data;
	byte op = calc->mem[calc->hw.mem_ltop(calc, w->pc)];
	int prefix = op == 0xCB || op == 0xDD || op == 0xED || op == 0xFD;

	if (!(prefix && addr == (word)(w->pc + 1)))
		w->pc = addr;
	return 0;
}

/* Runs after every write into the range. The core stops a run only when this returns non-zero. */
static int note_write(TilemCalc *calc, dword addr, void *data)
{
	McWatch *w = data;
	McWrite *r;
	dword at = calc->hw.mem_ltop(calc, addr);

	if (w->len == MAX_RECORDS) {
		w->lost++;
		return 0;
	}
	if (!w->rec && !(w->rec = malloc(MAX_RECORDS * sizeof *w->rec)))
		abort();
	r = &w->rec[w->len++];
	r->clock = calc->z80.clock;
	r->pc_physical = calc->hw.mem_ltop(calc, w->pc);
	r->addr = addr;
	r->pc = w->pc;
	r->value = calc->mem[at];
	return 0;
}

static void remove_hooks(TilemCalc *calc, McWatch *w)
{
	if (w->write_bp) {
		tilem_z80_remove_breakpoint(calc, w->write_bp);
		tilem_z80_remove_breakpoint(calc, w->exec_bp);
		w->write_bp = w->exec_bp = 0;
	}
}

void mc_watch_arm(TilemCalc *calc, McWatch **slot, uint16_t start, uint32_t len)
{
	McWatch *w = *slot;

	if (!w && !(w = calloc(1, sizeof *w)))
		abort();
	remove_hooks(calc, w);
	w->len = w->lost = 0;
	w->pc = calc->z80.r.pc.w.l;
	w->write_bp = tilem_z80_add_breakpoint(calc, TILEM_BREAK_MEM_WRITE, start, start + len - 1, 0xffff, note_write, w);
	w->exec_bp = tilem_z80_add_breakpoint(calc, TILEM_BREAK_MEM_EXEC, 0, 0xffff, 0xffff, note_pc, w);
	*slot = w;
}

void mc_watch_disarm(TilemCalc *calc, McWatch *w)
{
	if (w)
		remove_hooks(calc, w);
}

void mc_watch_run(TilemCalc *calc, McWatch *w)
{
	if (w && w->write_bp)
		w->pc = calc->z80.r.pc.w.l;
}

size_t mc_watch_len(const McWatch *w) { return w ? w->len : 0; }

size_t mc_watch_take(McWatch *w, McWrite *out, size_t cap, size_t *lost)
{
	size_t n = 0;

	*lost = 0;
	if (!w)
		return 0;
	n = w->len < cap ? w->len : cap;
	if (n)
		memcpy(out, w->rec, n * sizeof *out);
	*lost = w->lost;
	w->len = w->lost = 0;
	return n;
}

void mc_watch_free(TilemCalc *calc, McWatch *w)
{
	if (!w)
		return;
	remove_hooks(calc, w);
	free(w->rec);
	free(w);
}
