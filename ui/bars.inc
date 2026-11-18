; Bars: the title, the soft keys and the list row.

; ui_title   in:  HL = left text, DE = right text
;            left at column 1, right ending at column 94, the whole row inverted; neither is cut
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
	jp ui_invert_row

; ui_list_row   in:  HL = left text, DE = right text, B = row 0 to 7, C = 0 unselected
;               left at column 1, right ending at column 94, the row inverted when C is not 0
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

; The font the soft key labels are set in; the font in use is restored afterwards.
#define ui_key_font ui_font

; ui_soft_keys   in:  HL = five little endian pointers to label strings; an empty string leaves
;                     its key blank
;                a label is centred in its key's box on row 7, rounding the left margin up,
;                and the box is inverted; a label wider than its box is not handled
;                destroys AF BC DE HL
ui_soft_keys:
	ld de,(ui_fontp)
	push de
	ld de,ui_key_font-SLOT
	ld (ui_fontp),de
	ld (ui_kp),hl
	ld hl,sk_boxes
	ld (ui_kb),hl
	ld a,5
	ld (ui_kn),a
sk_next:
	ld hl,(ui_kp)
	ld e,(hl)
	inc hl
	ld d,(hl)
	inc hl
	ld (ui_kp),hl
	ld a,(de)
	or a
	jr z,sk_skip
	ex de,hl
	push hl
	call ui_text_width
	ld d,l
	ld hl,(ui_kb)
	ld b,(hl)
	inc hl
	ld c,(hl)
	ld a,c
	sub b
	sub d
	inc a
	srl a
	add a,b
	ld e,a
	ld d,0
	pop hl
	ld b,UI_SOFT_ROW
	call ui_text
	ld hl,(ui_kb)
	ld b,(hl)
	inc hl
	ld a,(hl)
	sub b
	ld d,a
	ld c,UI_SOFT_ROW*UI_ROW
	ld e,UI_ROW
	call ui_invert
sk_skip:
	ld hl,(ui_kb)
	inc hl
	inc hl
	ld (ui_kb),hl
	ld hl,ui_kn
	dec (hl)
	jr nz,sk_next
	pop hl
	ld (ui_fontp),hl
	ret

sk_boxes:
	.db (UI_WIDTH*0+2)/5,(UI_WIDTH*1+2)/5-1
	.db (UI_WIDTH*1+2)/5,(UI_WIDTH*2+2)/5-1
	.db (UI_WIDTH*2+2)/5,(UI_WIDTH*3+2)/5-1
	.db (UI_WIDTH*3+2)/5,(UI_WIDTH*4+2)/5-1
	.db (UI_WIDTH*4+2)/5,(UI_WIDTH*5+2)/5-1
