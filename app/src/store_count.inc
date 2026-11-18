; The count pass of store.inc, which includes this file.
; Counting runs in blocks of at most 63 table bytes, so one 8-bit counter per value, in OP2,
; cannot overflow, and adds the counters to the caller's words after each block.
CNT_BUF   = OP3
CNT_SRC   = OP3 + 2
CNT_LEFT  = OP3 + 4
CNT_TAIL  = OP3 + 6
CNT_BLOCK = 63
CNT_C0    = OP2

store_count:
	push hl
	call store_data
	pop de
	or a
	ret nz
	ld (CNT_BUF), de
	ld a, c
	and 3
	ld (CNT_TAIL), a
	srl b
	rr c
	srl b
	rr c
	ld (CNT_LEFT), bc
	ld de, STORE_HEADER
	add hl, de
	ld (CNT_SRC), hl
	ld hl, (CNT_BUF)
	ld d, h
	ld e, l
	inc de
	ld (hl), 0
	ld bc, 7
	ldir
	ld hl, CNT_C0
	ld d, h
	ld e, l
	inc de
	ld (hl), 0
	ld bc, 3
	ldir
sc_block:
	ld bc, (CNT_LEFT)
	ld a, b
	or c
	jr z, sc_tail
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
	call sc_flush
	jr sc_block
sc_tail:
	ld a, (CNT_TAIL)
	or a
	jr z, sc_done
	ld b, a
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
	call sc_flush
sc_done:
	xor a
	ret

; Adds the four block counters to the words of the caller's buffer and clears them.
sc_flush:
	ld hl, (CNT_BUF)
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
