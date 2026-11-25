; Text for the canvas. app_stage_block copies a screen's block of constant strings (strings.inc)
; into APP_STAGE, where the canvas can read it with its own page mapped, unless the area already
; holds that block. app_digit turns a nibble into the glyph code of its hex digit, for a status
; shown as two digits, app_dec16 writes a number in decimal and app_strcpy copies a string of glyph
; codes, and app_label_text writes an assertion's label as glyph codes. The glyph codes are the main
; font's (UI_FONT_CH_ equates of font.inc).

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

; In: HL = a number, DE = where to write, five bytes it may use. Writes its decimal digits as glyph
; codes with no leading zero, "0" for 0, and no terminator; the rest of the five bytes hold digit
; glyphs with no meaning. Out: DE = the address after the last digit. Destroys AF BC HL.
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

APP_LABEL_MAX	.equ 48

#ifdef LIBRARY

; In: HL = an assertion number, DE = where to write APP_LABEL_MAX + 1 bytes. Out: A = 0 and the first
; APP_LABEL_MAX characters of its label as glyph codes ended by 0, a character with no glyph being
; the question mark, or the reader's status with nothing written. Reads through FM_SCRATCH.
; Destroys BC DE HL.
app_label_text:
	push de
	ld de, FM_SCRATCH
	call far_lib_label
	pop de
	or a
	ret nz
	ld a, b
	cp APP_LABEL_MAX + 1
	jr c, al_count
	ld a, APP_LABEL_MAX
al_count:
	ld b, a
	ld hl, FM_SCRATCH
al_glyph:
	ld a, b
	or a
	jr z, al_cut
	push bc
	ld a, (hl)
	inc hl
	push hl
	sub 20h
	cp 95
	jr c, al_known
	ld a, 3Fh - 20h
al_known:
	ld c, a
	ld b, 0
	ld hl, app_glyphs
	add hl, bc
	ld a, (hl)
	ld (de), a
	inc de
	pop hl
	pop bc
	dec b
	jr al_glyph
al_cut:
	xor a
	ld (de), a
	ret

; The glyph code of each printable ASCII character from the space to the tilde.
app_glyphs:
	.db UI_FONT_CH_20, UI_FONT_CH_21, UI_FONT_CH_22, UI_FONT_CH_23, UI_FONT_CH_24, UI_FONT_CH_25
	.db UI_FONT_CH_26, UI_FONT_CH_27, UI_FONT_CH_28, UI_FONT_CH_29, UI_FONT_CH_2A, UI_FONT_CH_2B
	.db UI_FONT_CH_2C, UI_FONT_CH_2D, UI_FONT_CH_2E, UI_FONT_CH_2F, UI_FONT_CH_30, UI_FONT_CH_31
	.db UI_FONT_CH_32, UI_FONT_CH_33, UI_FONT_CH_34, UI_FONT_CH_35, UI_FONT_CH_36, UI_FONT_CH_37
	.db UI_FONT_CH_38, UI_FONT_CH_39, UI_FONT_CH_3A, UI_FONT_CH_3B, UI_FONT_CH_3C, UI_FONT_CH_3D
	.db UI_FONT_CH_3E, UI_FONT_CH_3F, UI_FONT_CH_40, UI_FONT_CH_41, UI_FONT_CH_42, UI_FONT_CH_43
	.db UI_FONT_CH_44, UI_FONT_CH_45, UI_FONT_CH_46, UI_FONT_CH_47, UI_FONT_CH_48, UI_FONT_CH_49
	.db UI_FONT_CH_4A, UI_FONT_CH_4B, UI_FONT_CH_4C, UI_FONT_CH_4D, UI_FONT_CH_4E, UI_FONT_CH_4F
	.db UI_FONT_CH_50, UI_FONT_CH_51, UI_FONT_CH_52, UI_FONT_CH_53, UI_FONT_CH_54, UI_FONT_CH_55
	.db UI_FONT_CH_56, UI_FONT_CH_57, UI_FONT_CH_58, UI_FONT_CH_59, UI_FONT_CH_5A, UI_FONT_CH_5B
	.db UI_FONT_CH_5C, UI_FONT_CH_5D, UI_FONT_CH_5E, UI_FONT_CH_5F, UI_FONT_CH_60, UI_FONT_CH_61
	.db UI_FONT_CH_62, UI_FONT_CH_63, UI_FONT_CH_64, UI_FONT_CH_65, UI_FONT_CH_66, UI_FONT_CH_67
	.db UI_FONT_CH_68, UI_FONT_CH_69, UI_FONT_CH_6A, UI_FONT_CH_6B, UI_FONT_CH_6C, UI_FONT_CH_6D
	.db UI_FONT_CH_6E, UI_FONT_CH_6F, UI_FONT_CH_70, UI_FONT_CH_71, UI_FONT_CH_72, UI_FONT_CH_73
	.db UI_FONT_CH_74, UI_FONT_CH_75, UI_FONT_CH_76, UI_FONT_CH_77, UI_FONT_CH_78, UI_FONT_CH_79
	.db UI_FONT_CH_7A, UI_FONT_CH_7B, UI_FONT_CH_7C, UI_FONT_CH_7D, UI_FONT_CH_7E
#endif
