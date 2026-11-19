; Bars: the title, the soft keys and the list row.

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

; ui_soft_keys   in:  HL = five little endian pointers to label strings encoded for the key font
;                     table ui_key_font, not the main one; an empty string leaves its key blank
;                a label is centred in its key's box on row 7, rounding the left margin up,
;                and the box is inverted below the row's top scanline; a label wider than its box is not handled
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
	inc hl
	inc hl
	call sk_flip
sk_skip:
	ld hl,(ui_kb)
	ld de,SK_RECORD
	add hl,de
	ld (ui_kb),hl
	ld hl,ui_kn
	dec (hl)
	jr nz,sk_next
	pop hl
	ld (ui_fontp),hl
	ret

; Flips a key's box from its record, HL = the record past its two x bytes: seven scanlines, with
; one or two whole bytes between the edge bytes. Destroys AF BC DE HL.
#macro FLIP3
	ld a,(hl)
	xor c
	ld (hl),a
	inc hl
	ld a,(hl)
	cpl
	ld (hl),a
	inc hl
	ld a,(hl)
	xor b
	ld (hl),a
	add hl,de
#endmacro
#macro FLIP4
	ld a,(hl)
	xor c
	ld (hl),a
	inc hl
	ld a,(hl)
	cpl
	ld (hl),a
	inc hl
	ld a,(hl)
	cpl
	ld (hl),a
	inc hl
	ld a,(hl)
	xor b
	ld (hl),a
	add hl,de
#endmacro
sk_flip:
	ld e,(hl)
	inc hl
	ld d,(hl)
	inc hl
	ld c,(hl)
	inc hl
	ld a,(hl)
	inc hl
	ld b,(hl)
	ex de,hl
	dec a
	jr nz,sk_flip4
	ld de,UI_STRIDE-2
	FLIP3
	FLIP3
	FLIP3
	FLIP3
	FLIP3
	FLIP3
	FLIP3
	ret
sk_flip4:
	ld de,UI_STRIDE-3
	FLIP4
	FLIP4
	FLIP4
	FLIP4
	FLIP4
	FLIP4
	FLIP4
	ret

; A key box record: its first and last column, the address of its top left byte in the frame
; (the second scanline of the soft key row, which leaves the first one clear),
; the mask of the first byte, the number of whole bytes between, and the mask of the last. The
; box is inverted up to the column before its last.
SK_RECORD               .equ 7
#macro BOX(a,b)
#if (((b-1)>>3)-(a>>3)-1) < 1
 .error "a soft key box must span one or two whole bytes between its edge bytes"
#endif
#if (((b-1)>>3)-(a>>3)-1) > 2
 .error "a soft key box must span one or two whole bytes between its edge bytes"
#endif
	.db a,b
	.dw UI_FRAME+((UI_SOFT_ROW*UI_ROW+1)*UI_STRIDE)+(a>>3)
	.db $FF>>(a&7),((b-1)>>3)-(a>>3)-1,($FF<<(7-((b-1)&7)))&$FF
#endmacro
sk_boxes:
	BOX((UI_WIDTH*0+2)/5,(UI_WIDTH*1+2)/5-1)
	BOX((UI_WIDTH*1+2)/5,(UI_WIDTH*2+2)/5-1)
	BOX((UI_WIDTH*2+2)/5,(UI_WIDTH*3+2)/5-1)
	BOX((UI_WIDTH*3+2)/5,(UI_WIDTH*4+2)/5-1)
	BOX((UI_WIDTH*4+2)/5,(UI_WIDTH*5+2)/5-1)
