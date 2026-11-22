; Text for the canvas. app_stage_block copies a screen's block of constant strings (strings.inc)
; into APP_STAGE, where the canvas can read it with its own page mapped, unless the area already
; holds that block. app_digit turns a nibble into the glyph code of its hex digit, for a status
; shown as two digits. With LIBRARY, app_codes turns the ASCII a reader accessor returns into glyph
; codes in place. The glyph codes are the main font's (UI_FONT_CH_ equates of font.inc).

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

#ifdef LIBRARY
; In: B = the length of the ASCII text at HL. Writes the glyph codes of the characters over it and a
; zero after them. A character with no glyph in the table is a question mark.
app_codes:
	ld a, b
	or a
	jr z, app_codes_end
app_code:
	ld a, (hl)
	sub 20h
	cp 95
	jr c, app_code_in
	ld a, 3Fh - 20h
app_code_in:
	push hl
	ld e, a
	ld d, 0
	ld hl, app_ascii
	add hl, de
	ld a, (hl)
	pop hl
	ld (hl), a
	inc hl
	djnz app_code
app_codes_end:
	ld (hl), 0
	ret

; The glyph code of each character from the space to the tilde.
app_ascii:
	.db UI_FONT_CH_20, UI_FONT_CH_21, UI_FONT_CH_22, UI_FONT_CH_23, UI_FONT_CH_24, UI_FONT_CH_25, UI_FONT_CH_26, UI_FONT_CH_27
	.db UI_FONT_CH_28, UI_FONT_CH_29, UI_FONT_CH_2A, UI_FONT_CH_2B, UI_FONT_CH_2C, UI_FONT_CH_2D, UI_FONT_CH_2E, UI_FONT_CH_2F
	.db UI_FONT_CH_30, UI_FONT_CH_31, UI_FONT_CH_32, UI_FONT_CH_33, UI_FONT_CH_34, UI_FONT_CH_35, UI_FONT_CH_36, UI_FONT_CH_37
	.db UI_FONT_CH_38, UI_FONT_CH_39, UI_FONT_CH_3A, UI_FONT_CH_3B, UI_FONT_CH_3C, UI_FONT_CH_3D, UI_FONT_CH_3E, UI_FONT_CH_3F
	.db UI_FONT_CH_40, UI_FONT_CH_41, UI_FONT_CH_42, UI_FONT_CH_43, UI_FONT_CH_44, UI_FONT_CH_45, UI_FONT_CH_46, UI_FONT_CH_47
	.db UI_FONT_CH_48, UI_FONT_CH_49, UI_FONT_CH_4A, UI_FONT_CH_4B, UI_FONT_CH_4C, UI_FONT_CH_4D, UI_FONT_CH_4E, UI_FONT_CH_4F
	.db UI_FONT_CH_50, UI_FONT_CH_51, UI_FONT_CH_52, UI_FONT_CH_53, UI_FONT_CH_54, UI_FONT_CH_55, UI_FONT_CH_56, UI_FONT_CH_57
	.db UI_FONT_CH_58, UI_FONT_CH_59, UI_FONT_CH_5A, UI_FONT_CH_5B, UI_FONT_CH_5C, UI_FONT_CH_5D, UI_FONT_CH_5E, UI_FONT_CH_5F
	.db UI_FONT_CH_60, UI_FONT_CH_61, UI_FONT_CH_62, UI_FONT_CH_63, UI_FONT_CH_64, UI_FONT_CH_65, UI_FONT_CH_66, UI_FONT_CH_67
	.db UI_FONT_CH_68, UI_FONT_CH_69, UI_FONT_CH_6A, UI_FONT_CH_6B, UI_FONT_CH_6C, UI_FONT_CH_6D, UI_FONT_CH_6E, UI_FONT_CH_6F
	.db UI_FONT_CH_70, UI_FONT_CH_71, UI_FONT_CH_72, UI_FONT_CH_73, UI_FONT_CH_74, UI_FONT_CH_75, UI_FONT_CH_76, UI_FONT_CH_77
	.db UI_FONT_CH_78, UI_FONT_CH_79, UI_FONT_CH_7A, UI_FONT_CH_7B, UI_FONT_CH_7C, UI_FONT_CH_7D, UI_FONT_CH_7E
#endif
