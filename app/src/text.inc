; Text for the canvas. app_stage_block copies a screen's block of constant strings (strings.inc)
; into APP_STAGE, where the canvas can read it with its own page mapped, unless the area already
; holds that block. app_digit turns a nibble into the glyph code of its hex digit, for a status
; shown as two digits, app_dec16 writes a number in decimal and app_strcpy copies a string of glyph
; codes. The glyph codes are the main font's (UI_FONT_CH_ equates of font.inc).

; In: HL = a block on this page, BC = its length. Copies it to APP_STAGE_DATA and sets the tag to HL,
; unless the tag is HL already; a length of 0 does nothing. A block longer than APP_STAGE_DATA_SIZE
; is refused whole: nothing is copied and the tag is cleared. Out: carry set when refused, else
; reset. Destroys AF BC DE HL.
app_stage_block:
	ld a, b
	or c
	ret z
	ld a, b
	or a
	jr nz, stage_refuse
	ld a, c
	cp APP_STAGE_DATA_SIZE + 1
	jr nc, stage_refuse
	ld de, (APP_STAGE_TAG)
	ld a, l
	cp e
	jr nz, stage_copy
	ld a, h
	cp d
	ret z
stage_copy:
	ld de, APP_STAGE_DATA
	push hl
	ldir
	pop hl
	ld (APP_STAGE_TAG), hl
	or a
	ret
stage_refuse:
	ld hl, 0
	ld (APP_STAGE_TAG), hl
	scf
	ret

; In: A, low nibble. Out: A = the glyph code of its hex digit. Keeps B.
app_digit:
	and 0Fh
	ld e, a
	ld d, 0
	ld hl, app_hex
	add hl, de
	ld a, (hl)
	ret

; In: A = a byte, HL = where to write. Writes the glyph codes of its two hex digits and a 0 after
; them. Out: HL = the address of the 0. Destroys AF BC DE.
app_hex_byte:
	ld b, a
	rrca
	rrca
	rrca
	rrca
	push hl
	call app_digit
	pop hl
	ld (hl), a
	inc hl
	ld a, b
	push hl
	call app_digit
	pop hl
	ld (hl), a
	inc hl
	ld (hl), 0
	ret

app_hex:
	.db UI_FONT_CH_30, UI_FONT_CH_31, UI_FONT_CH_32, UI_FONT_CH_33, UI_FONT_CH_34, UI_FONT_CH_35, UI_FONT_CH_36, UI_FONT_CH_37, UI_FONT_CH_38, UI_FONT_CH_39, UI_FONT_CH_41, UI_FONT_CH_42, UI_FONT_CH_43, UI_FONT_CH_44, UI_FONT_CH_45, UI_FONT_CH_46

; In: HL = a number, DE = where to write. Writes its decimal digits as glyph codes with no leading
; zero, "0" for 0, and no terminator. Out: DE = the address after the last digit.
; Destroys AF BC HL.
app_dec16:
	push de
	ld bc, -10000
	call dec_digit
	ld bc, -1000
	call dec_digit
	ld bc, -100
	call dec_digit
	ld bc, -10
	call dec_digit
	ld a, l
	add a, UI_FONT_CH_30
	ld (de), a
	inc de
	pop hl
	push hl
	ld b, 4
dec_zero:
	ld a, (hl)
	cp UI_FONT_CH_30
	jr nz, dec_shift
	inc hl
	djnz dec_zero
dec_shift:
	ex de, hl
	or a
	sbc hl, de
	ld b, h
	ld c, l
	ex de, hl
	pop de
	ldir
	ret

; In: HL = a number below the power in BC (negated), DE = where to write. Writes the digit and
; leaves the remainder in HL.
dec_digit:
	ld a, UI_FONT_CH_30 - 1
dec_sub:
	inc a
	add hl, bc
	jr c, dec_sub
	sbc hl, bc
	ld (de), a
	inc de
	ret

; In: HL = a string of glyph codes, DE = where to write. Copies it with its 0. Out: HL and DE are
; after the 0. Destroys AF.
app_strcpy:
	ld a, (hl)
	ld (de), a
	inc hl
	inc de
	or a
	jr nz, app_strcpy
	ret
