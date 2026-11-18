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

; Contract: HL points at a zero-terminated string in the app page, which a bcall swaps out, so it
; is read here and passed on one character at a time.
putstr:
	ld a, (hl)
	or a
	ret z
	call putchar
	inc hl
	jr putstr

; Prints HL as five decimal digits.
putdec:
	ld de, 10000
	call digit
	ld de, 1000
	call digit
	ld de, 100
	call digit
	ld de, 10
	call digit
	ld hl, (S_WFIRST)
	ld a, l
	add a, '0'
	jp putchar

; Prints the quotient of HL by DE as one digit and leaves the remainder in scratch.
digit:
	ld b, 0
digit_sub:
	or a
	sbc hl, de
	jr c, digit_done
	inc b
	jr digit_sub
digit_done:
	add hl, de
	ld (S_WFIRST), hl
	ld a, '0'
	add a, b
	call putchar
	ld hl, (S_WFIRST)
	ret

; Prints HL as four hex digits.
puthex:
	ld (S_WFIRST), hl
	ld a, (S_WFIRST + 1)
	call hexbyte
	ld a, (S_WFIRST)

hexbyte:
	ld (S_WLAST), a
	rrca
	rrca
	rrca
	rrca
	call nibble
	ld a, (S_WLAST)

nibble:
	and 0Fh
	add a, '0'
	cp '9' + 1
	jr c, nibble_put
	add a, 'A' - '9' - 1
nibble_put:
	jp putchar

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
