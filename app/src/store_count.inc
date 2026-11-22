; The count pass of store.inc, which includes this file.
; One pass over the table fills one result per end of a short ascending list: the counts of the
; entries below that end. Whole bytes are counted into a running total, in blocks of at most 63
; table bytes so each 8-bit counter in OP2 cannot overflow, and the total is copied into the
; end's result; the entries of a last partial byte are counted into the copy only, so the next
; end starts from whole bytes again.
CNT_BLOCK    = 63
CNT_MAX_ENDS = 8
CNT_C0       = OP2
CNT_SRC      = OP3
CNT_LEFT     = OP3 + 2
CNT_POS      = OP3 + 4
CNT_COUNT    = OP3 + 6
CNT_TAIL     = OP3 + 8
CNT_N        = OP3 + 9
CNT_ACC      = OP4
CNT_ONE      = OP4 + 8
CNT_LIST     = OP5
CNT_OUT      = OP5 + 2

store_count:
	push hl
	call store_data
	pop de
	or a
	ret nz
	ld (CNT_ONE + 1), bc
	jr sc_one

store_count_to:
	push hl
	push de
	call store_data
	pop de
	or a
	jr nz, sct_fail
	ld (CNT_ONE + 1), de
	pop de
sc_one:
	ld a, 1
	ld (CNT_ONE), a
	ld ix, CNT_ONE
	jr sc_pass
sct_fail:
	pop hl
	ret

store_count_ends:
	push de
	push hl
	call store_data
	pop ix
	pop de
	or a
	ret nz
	jr sc_pass
sc_range:
	ld a, ST_RANGE
	ret

; In: HL = the header, BC = the assertion count, DE = the results, IX = the list. Out: A.
sc_pass:
	ld (CNT_OUT), de
	ld (CNT_COUNT), bc
	ld de, STORE_HEADER
	add hl, de
	ld (CNT_SRC), hl
	ld a, (ix + 0)
	or a
	jr z, sc_range
	cp CNT_MAX_ENDS + 1
	jr nc, sc_range
	ld (CNT_N), a
	ld b, a
	inc ix
	ld (CNT_LIST), ix
	ld e, (ix + 0)
	ld d, (ix + 1)
	jr sc_bound
sc_rise:
	ld e, (ix + 0)
	ld d, (ix + 1)
	or a
	sbc hl, de
	jr nc, sc_range
sc_bound:
	push de
	ld hl, (CNT_COUNT)
	or a
	sbc hl, de
	pop hl
	jr c, sc_range
	inc ix
	inc ix
	djnz sc_rise
	ld hl, 0
	ld (CNT_POS), hl
	ld hl, CNT_ACC
	ld bc, 7
	call sc_clear
	ld hl, CNT_C0
	ld bc, 3
	call sc_clear
sc_end:
	ld hl, (CNT_LIST)
	ld e, (hl)
	inc hl
	ld d, (hl)
	inc hl
	ld (CNT_LIST), hl
	ld a, e
	and 3
	ld (CNT_TAIL), a
	srl d
	rr e
	srl d
	rr e
	ld hl, (CNT_POS)
	ld (CNT_POS), de
	ex de, hl
	or a
	sbc hl, de
	ld (CNT_LEFT), hl
	call sc_bytes
	ld hl, CNT_ACC
	ld de, (CNT_OUT)
	ld bc, 8
	ldir
	ld (CNT_OUT), de
	ld hl, -8
	add hl, de
	call sc_tail
	ld hl, CNT_N
	dec (hl)
	jr nz, sc_end
	xor a
	ret

; In: HL = the first byte, BC = its count less one. Zeroes the bytes.
sc_clear:
	ld (hl), 0
	ld d, h
	ld e, l
	inc de
	ldir
	ret

; Counts CNT_LEFT whole table bytes from CNT_SRC into the running total.
sc_bytes:
	ld bc, (CNT_LEFT)
	ld a, b
	or c
	ret z
	ld a, b
	or a
	jr nz, sc_full
	ld a, c
	cp CNT_BLOCK
	jr c, sc_part
sc_full:
	ld a, CNT_BLOCK
sc_part:
	ld e, a
	ld d, 0
	ld hl, (CNT_LEFT)
	or a
	sbc hl, de
	ld (CNT_LEFT), hl
	ld b, a
	ld de, (CNT_SRC)
	ld h, CNT_C0 >> 8
sc_byte:
	ld a, (de)
	inc de
	ld c, a
	and 3
	add a, CNT_C0 & 0FFh
	ld l, a
	inc (hl)
	ld a, c
	rrca
	rrca
	ld c, a
	and 3
	add a, CNT_C0 & 0FFh
	ld l, a
	inc (hl)
	ld a, c
	rrca
	rrca
	ld c, a
	and 3
	add a, CNT_C0 & 0FFh
	ld l, a
	inc (hl)
	ld a, c
	rrca
	rrca
	and 3
	add a, CNT_C0 & 0FFh
	ld l, a
	inc (hl)
	djnz sc_byte
	ld (CNT_SRC), de
	ld hl, CNT_ACC
	call sc_flush
	jr sc_bytes

; In: HL = a result. Adds the first CNT_TAIL entries of the byte at CNT_SRC to it.
sc_tail:
	ld a, (CNT_TAIL)
	or a
	ret z
	ld b, a
	push hl
	ld de, (CNT_SRC)
	ld a, (de)
	ld c, a
	ld h, CNT_C0 >> 8
sc_field:
	ld a, c
	and 3
	add a, CNT_C0 & 0FFh
	ld l, a
	inc (hl)
	srl c
	srl c
	djnz sc_field
	pop hl

; In: HL = four words. Adds the four block counters to them and clears the counters.
sc_flush:
	ld de, CNT_C0
	ld b, 4
sc_flush_one:
	ld a, (de)
	add a, (hl)
	ld (hl), a
	inc hl
	jr nc, sc_flush_low
	inc (hl)
sc_flush_low:
	inc hl
	xor a
	ld (de), a
	inc de
	djnz sc_flush_one
	ret
