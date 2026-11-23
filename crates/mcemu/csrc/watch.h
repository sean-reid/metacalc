#ifndef MCEMU_WATCH_H
#define MCEMU_WATCH_H

#include <stddef.h>
#include <stdio.h>
#include <stdint.h>

#include "tilem.h"

typedef struct McWatch McWatch;

/* One write the CPU made into the watched range. */
typedef struct {
	uint32_t clock;
	uint32_t pc_physical;
	uint16_t addr;
	uint16_t pc;
	uint8_t value;
} McWrite;

/* Arms a watch on `len` logical addresses from `start`, replacing any watch in `*slot`. */
void mc_watch_arm(TilemCalc *calc, McWatch **slot, uint16_t start, uint32_t len);
/* Stops recording and keeps what was recorded. */
void mc_watch_disarm(TilemCalc *calc, McWatch *w);
/* Called as a run begins, so a write by the first instruction has its address. */
void mc_watch_run(TilemCalc *calc, McWatch *w);
size_t mc_watch_len(const McWatch *w);
/* Copies up to `cap` records out and clears the record; returns the count lost to the cap since the last take in *lost. */
size_t mc_watch_take(McWatch *w, McWrite *out, size_t cap, size_t *lost);
void mc_watch_free(TilemCalc *calc, McWatch *w);

#endif
