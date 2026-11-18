; Contract: the caller defines putchar (A = character, 0Ah = new line) and the stack is usable. Prints
; the results from scratch; digits go out one at a time with the running value kept in scratch,
; because the output routine may change registers.
show_results:
	ld hl, title
	call putstr
	ld a, 0Ah
	call putchar
	ld hl, text_write
	call putstr
	ld hl, (S_WBAD)
	ld a, h
	or l
	jr nz, write_bad
	ld hl, text_ok
	call putstr
	jr write_done
write_bad:
	ld hl, text_bad
	call putstr
	ld hl, (S_WBAD)
	call putdec
write_done:
	ld a, 0Ah
	call putchar
	ld a, (S_CHECKED)
	or a
	jr nz, checked
	ld hl, text_first
	jp putstr
checked:
	ld hl, (S_LOST)
	ld a, h
	or l
	jr nz, lost
	ld hl, text_kept
	jp putstr
lost:
	ld hl, text_lost
	call putstr
	ld hl, (S_LOST)
	call putdec
	ld a, 0Ah
	call putchar
	ld hl, (S_FIRST)
	call puthex
	ld a, '-'
	call putchar
	ld hl, (S_LAST)
	jp puthex

P_TMP  = S_WFIRST
P_TMP2 = S_WLAST
P_STR  = S_TPTR
#include "printnum.inc"

title:
	.db "sparechk v", '0' + VERSION, 0
text_write:
	.db "write ", 0
text_ok:
	.db "ok", 0
text_bad:
	.db "bad ", 0
text_first:
	.db "first run", 0
text_kept:
	.db "kept all", 0
text_lost:
	.db "lost ", 0
