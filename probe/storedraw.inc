; The log's hex dump on the screen, for storecheck.z80, which includes this file.
; Draws the log as hex digits, 24 to a line, ten lines.
LOG_SHOWN = 120

draw_log:
	ld hl, plotSScreen
	ld de, plotSScreen + 1
	ld bc, 767
	ld (hl), 0
	ldir
	ld hl, LOG + 2
	ld (S_DRAW), hl
	ld hl, 0
	ld (S_DRAW + 2), hl
dl_next:
	ld hl, (S_DRAW + 2)
	ld de, LOG_SHOWN
	or a
	sbc hl, de
	jr nc, dl_show
	ld hl, (S_DRAW)
	ld de, (LOG)
	or a
	sbc hl, de
	jr nc, dl_show
	ld hl, (S_DRAW)
	ld a, (hl)
	inc hl
	ld (S_DRAW), hl
	ld (S_DRAW + 4), a
	ld a, (S_DRAW + 2)
	ld d, 0
dl_div:
	cp 12
	jr c, dl_have
	sub 12
	inc d
	jr dl_div
dl_have:
	add a, a
	ld (S_DRAW + 6), a
	ld a, d
	ld (S_DRAW + 5), a
	ld a, (S_DRAW + 4)
	rrca
	rrca
	rrca
	rrca
	and 0Fh
	call draw_digit
	ld hl, S_DRAW + 6
	inc (hl)
	ld a, (S_DRAW + 4)
	and 0Fh
	call draw_digit
	ld hl, (S_DRAW + 2)
	inc hl
	ld (S_DRAW + 2), hl
	jr dl_next
dl_show:
	bcall(_GrBufCpy)
	ret

; In: A = a digit, S_DRAW + 5 = the line, S_DRAW + 6 = the cell, four pixels wide, in the line.
draw_digit:
	ld l, a
	ld h, 0
	ld e, l
	ld d, h
	add hl, hl
	add hl, hl
	add hl, de
	ld de, font
	add hl, de
	push hl
	ld a, (S_DRAW + 5)
	ld b, a
	ld hl, plotSScreen
	ld de, 72
	inc b
	jr dd_start
dd_line:
	add hl, de
dd_start:
	djnz dd_line
	ld a, (S_DRAW + 6)
	ld c, a
	srl a
	ld e, a
	ld d, 0
	add hl, de
	pop de
	ld b, 5
dd_row:
	ld a, (de)
	inc de
	bit 0, c
	jr z, dd_put
	srl a
	srl a
	srl a
	srl a
dd_put:
	or (hl)
	ld (hl), a
	push de
	ld de, 12
	add hl, de
	pop de
	djnz dd_row
	ret

; 3 by 5 digits, one byte a row with the pixels in the top three bits.
font:
	.db 0E0h, 0A0h, 0A0h, 0A0h, 0E0h
	.db 040h, 0C0h, 040h, 040h, 0E0h
	.db 0E0h, 020h, 0E0h, 080h, 0E0h
	.db 0E0h, 020h, 0E0h, 020h, 0E0h
	.db 0A0h, 0A0h, 0E0h, 020h, 020h
	.db 0E0h, 080h, 0E0h, 020h, 0E0h
	.db 0E0h, 080h, 0E0h, 0A0h, 0E0h
	.db 0E0h, 020h, 020h, 020h, 020h
	.db 0E0h, 0A0h, 0E0h, 0A0h, 0E0h
	.db 0E0h, 0A0h, 0E0h, 020h, 0E0h
	.db 0E0h, 0A0h, 0E0h, 0A0h, 0A0h
	.db 0C0h, 0A0h, 0C0h, 0A0h, 0C0h
	.db 0E0h, 080h, 080h, 080h, 0E0h
	.db 0C0h, 0A0h, 0A0h, 0A0h, 0C0h
	.db 0E0h, 080h, 0E0h, 080h, 0E0h
	.db 0E0h, 080h, 0E0h, 080h, 080h
