; The title and the list row. Assembled in the empty block after the one byte forms (glyph.inc).

; ui_title   in:  HL = left text, DE = right text
;            left at column 1, right ending at column 94, all eight scanlines of the row inverted;
;            neither is cut
;            destroys AF BC DE HL
ui_title:
	push de
	ld de,UI_MARGIN
	ld b,0
	call ui_text
	pop hl
	ld b,0
	call ui_text_right
	xor a
	call ui_invert_row
	ld hl,UI_FRAME
	ld b,UI_STRIDE
ut_top:
	ld a,(hl)
	cpl
	ld (hl),a
	inc hl
	djnz ut_top
	ret

; ui_list_row   in:  HL = left text, DE = right text, B = row 0 to 7, C = 0 unselected
;               left at column 1, right ending at column 94, the row inverted below its top
;               scanline when C is not 0
;               destroys AF BC DE HL
ui_list_row:
	push bc
	push de
	ld de,UI_MARGIN
	call ui_text
	pop hl
	pop bc
	push bc
	call ui_text_right
	pop bc
	ld a,c
	or a
	ret z
	ld a,b
	jp ui_invert_row
