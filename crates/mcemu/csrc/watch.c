#include <stdlib.h>
#include <string.h>

#include "watch.h"

#define MAX_RECORDS 65536

struct McWatch {
	int write_bp;
	int exec_bp;
	int op_bp;
	int boundary;
	uint16_t pc;
	uint16_t start;
	uint32_t span;
	McWrite *rec;
	size_t len;
	size_t lost;
};

/* Runs after each instruction executes, before the fetch of the next one. */
static int note_boundary(TilemCalc *calc, dword op, void *data)
{
	(void)calc;
	(void)op;
	((McWatch *)data)->boundary = 1;
	return 0;
}

/* Runs for the next instruction's address after an instruction and for the byte after each prefix.
   Only the first follows an instruction, so only it moves `pc`. */
static int note_pc(TilemCalc *calc, dword addr, void *data)
{
	McWatch *w = data;

	(void)calc;
	if (w->boundary) {
		w->pc = addr;
		w->boundary = 0;
	}
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
		tilem_z80_remove_breakpoint(calc, w->op_bp);
		w->write_bp = w->exec_bp = w->op_bp = 0;
	}
}

static void add_hooks(TilemCalc *calc, McWatch *w)
{
	w->pc = calc->z80.r.pc.w.l;
	w->boundary = 0;
	w->write_bp = tilem_z80_add_breakpoint(calc, TILEM_BREAK_MEM_WRITE, w->start, w->start + w->span - 1, 0xffff, note_write, w);
	w->exec_bp = tilem_z80_add_breakpoint(calc, TILEM_BREAK_MEM_EXEC, 0, 0xffff, 0xffff, note_pc, w);
	w->op_bp = tilem_z80_add_breakpoint(calc, TILEM_BREAK_EXECUTE, 0, 0xffffffff, 0xffffffff, note_boundary, w);
}

void mc_watch_arm(TilemCalc *calc, McWatch **slot, uint16_t start, uint32_t len)
{
	McWatch *w = *slot;

	if (!w && !(w = calloc(1, sizeof *w)))
		abort();
	remove_hooks(calc, w);
	w->len = w->lost = 0;
	w->start = start;
	w->span = len;
	add_hooks(calc, w);
	*slot = w;
}

int mc_watch_suspend(TilemCalc *calc, McWatch *w)
{
	if (!w || !w->write_bp)
		return 0;
	remove_hooks(calc, w);
	return 1;
}

void mc_watch_resume(TilemCalc *calc, McWatch *w)
{
	if (w && !w->write_bp)
		add_hooks(calc, w);
}

void mc_watch_disarm(TilemCalc *calc, McWatch *w)
{
	if (w)
		remove_hooks(calc, w);
}

void mc_watch_run(TilemCalc *calc, McWatch *w)
{
	if (w && w->write_bp) {
		w->pc = calc->z80.r.pc.w.l;
		w->boundary = 0;
	}
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
