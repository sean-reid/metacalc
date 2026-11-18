; Contract: the including file defines putchar (A = character; HL survives it), P_TMP (2 bytes) and
; P_TMP2 (1 byte) as scratch outside the registers, and the stack is usable.

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
	ld hl, (P_TMP)
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
	ld (P_TMP), hl
	ld a, '0'
	add a, b
	call putchar
	ld hl, (P_TMP)
	ret

; Prints HL as four hex digits.
puthex:
	ld (P_TMP), hl
	ld a, (P_TMP + 1)
	call hexbyte
	ld a, (P_TMP)

hexbyte:
	ld (P_TMP2), a
	rrca
	rrca
	rrca
	rrca
	call nibble
	ld a, (P_TMP2)

nibble:
	and 0Fh
	add a, '0'
	cp '9' + 1
	jr c, nibble_put
	add a, 'A' - '9' - 1
nibble_put:
	jp putchar
