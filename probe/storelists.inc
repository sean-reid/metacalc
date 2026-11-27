; The commands that scan a list of assertion numbers against the table, included by storecheck.z80.

do_all_ones:
	call fetch
	ld c, a
	ld b, 0
	ld hl, (S_PTR)
	push hl
	add hl, bc
	add hl, bc
	ld (S_PTR), hl
	pop hl
	call store_all_ones
	ld (S_RES + 1), a
	ld a, b
	ld (S_RES + 2), a
	ld a, 3
	jp result_len

do_next_zero:
	call fetch_word
	push hl
	call fetch_word
	ex de, hl
	pop hl
	call store_next_zero
	ld (S_RES + 1), a
	ld (S_RES + 2), hl
	ld a, 4
	jp result_len

do_next_not_one:
	call fetch_word
	push hl
	call fetch_word
	ex de, hl
	pop hl
	call store_next_not_one
	ld (S_RES + 1), a
	ld (S_RES + 2), hl
	ld a, 4
	jp result_len

do_prev_not_one:
	call fetch_word
	call store_prev_not_one
	ld (S_RES + 1), a
	ld (S_RES + 2), hl
	ld a, 4
	jp result_len

do_least_not_one:
	call fetch_word
	push hl
	call fetch_word
	push hl
	call fetch
	ld c, a
	ld b, 0
	ld hl, (S_PTR)
	push hl
	add hl, bc
	add hl, bc
	ld (S_PTR), hl
	pop hl
	pop ix
	pop de
	call store_least_not_one
	ld (S_RES + 1), a
	ld (S_RES + 2), hl
	ld a, 4
	jp result_len
