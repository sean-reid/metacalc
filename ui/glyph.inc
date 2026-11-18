; Text: width and drawing of strings of glyph codes. Strings, the font table and the codes are
; described by mcglyph's z80 module.

; ui_set_font    in:  HL = a font table, as mcglyph z80 writes it
;                 every routine below reads its glyphs from the table set last
;                 destroys AF DE HL
ui_set_font:
	ld de,-SLOT
	add hl,de
	ld (ui_fontp),hl
	ret

; ui_text_width   in:  HL = string
;                 out: HL = width in pixels
;                 destroys AF BC DE
ui_text_width:
	ld de,0
tw_loop:
	ld a,(hl)
	or a
	jr z,tw_done
	inc hl
	push hl
	ld l,a
	ld h,0
	add hl,hl
	add hl,hl
	add hl,hl
	ld bc,(ui_fontp)
	add hl,bc
	ld c,(hl)
	pop hl
	ld a,e
	add a,c
	ld e,a
	jr nc,tw_carry
	inc d
tw_carry:
	inc de
	jr tw_loop
tw_done:
	ex de,hl
	ld a,h
	or l
	ret z
	dec hl
	ret

; ui_text_right     in:  HL = string, B = row
;                   out: DE = x after the string
;                   draws it ending at column 94, from 0 when it is wider
;                   destroys AF BC HL
ui_text_right:
	push hl
	push bc
	call ui_text_width
	ld de,UI_WIDTH-UI_MARGIN
	ex de,hl
	or a
	sbc hl,de
	jr nc,tr_x
	ld hl,0
tr_x:
	ex de,hl
	pop bc
	pop hl
	jr ui_text

; ui_text_centered  in:  HL = string, B = row
;                   out: DE = x after the string
;                   starts at (96 - width) / 2, or 0 when it is wider
;                   destroys AF BC HL
ui_text_centered:
	push hl
	push bc
	call ui_text_width
	ld de,UI_WIDTH
	ex de,hl
	or a
	sbc hl,de
	jr nc,tc_x
	ld hl,0
tc_x:
	srl h
	rr l
	ex de,hl
	pop bc
	pop hl

; ui_text   in:  HL = string, DE = x in pixels, B = text row 0 to 7; a row of 8 or more draws
;           nothing
;           out: DE = x after the last glyph, DE = x for an empty string
;           ORs the glyphs into the frame, one blank column between them, and drops pixels
;           right of column 95. Destroys AF BC HL.
ui_text:
	ld a,(hl)
	or a
	ret z
	ld a,b
	cp UI_TEXT_ROWS
	jp nc,ut_off
	push hl
	push de
	ld a,b
	call ui_row_addr
	ld (ui_rowbase),hl
	pop de
	pop hl
	ld a,d
	or a
	jr nz,ut_tail
	ld a,e
	cp UI_WIDTH
	jr nc,ut_tail
	jr ut_loop

; The loop runs with HL = the next glyph code, A = its x, below column 96. The string pointer
; and the x after the glyph wait on the stack while its rows are drawn.
ut_after:
	pop af
	pop hl
	cp UI_WIDTH
	jr nc,ut_clip
ut_loop:
	ld e,a
	ld a,(hl)
	inc hl
	or a
	jr z,ut_end
	ld d,a
	push hl
	rlca
	rlca
	rlca
	ld l,a
	and 7
	ld h,a
	xor l
	ld l,a
	ld bc,(ui_fontp)
	add hl,bc
	ld c,(hl)
	inc hl
	dec d
	jr z,ut_space
	ld a,e
	add a,c
	inc a
	push af
	ld a,e
	and 7
	ld b,a
	add a,c
	cp 9
	jr nc,ut_two
	ld a,b
	add a,a
	add a,16
ut_go:
	ex de,hl
	or ui_vtab&$FF
	ld c,a
	ld a,l
	rrca
	rrca
	rrca
	and $1F
	ld b,a
	ld h,ui_vtab>>8
	ld l,c
	ld a,(hl)
	inc hl
	ld h,(hl)
	ld l,a
	push hl
	ld hl,(ui_rowbase)
	ld c,b
	ld b,0
	add hl,bc
	ret
ut_two:
	ld a,c
	cp 9
	jr nc,ut_slow
	ld a,e
	cp 88
	jr nc,ut_slow
	ld a,b
	add a,a
	jr ut_go
ut_space:
	ld a,e
	add a,c
	inc a
	pop hl
	cp UI_WIDTH
	jr c,ut_loop
ut_clip:
	ld e,a
	ld d,0
ut_tail:
	ld a,(hl)
	or a
	jr z,ut_last
	push de
	call ui_text_width
	pop de
	add hl,de
	ex de,hl
	ret
ut_end:
	ld d,0
ut_last:
	dec de
	ret

; A row below the frame draws nothing and still returns the x after the string.
ut_off:
	push de
	call ui_text_width
	pop de
	add hl,de
	ex de,hl
	ret

; A glyph that is nine wide or spills out of the last byte of a row, HL = its rows, E = x,
; C = width: shifts each row as a 16 bit value and leaves the second byte alone in the last
; column.
ut_slow:
	ld (ui_src),hl
	ld a,c
	cp 9
	sbc a,a
	ld (ui_narrow),a
	ld a,e
	and 7
	ld (ui_shift),a
	ld a,e
	rrca
	rrca
	rrca
	and $1F
	ld (ui_col),a
	ld c,a
	ld b,0
	ld hl,(ui_rowbase)
	add hl,bc
	ld (ui_dst),hl
	ld a,7
	ld (ui_cnt),a
us_row:
	ld hl,(ui_src)
	ld d,(hl)
	ld e,0
	ld a,(ui_narrow)
	or a
	jr nz,us_second
	ld bc,8
	add hl,bc
	ld e,(hl)
us_second:
	ld hl,(ui_src)
	inc hl
	ld (ui_src),hl
	ld a,(ui_shift)
	or a
	jr z,us_ready
	ld b,a
us_shift:
	srl d
	rr e
	djnz us_shift
us_ready:
	ld hl,(ui_dst)
	ld a,(hl)
	or d
	ld (hl),a
	ld a,(ui_col)
	cp UI_STRIDE-1
	jr z,us_next
	inc hl
	ld a,(hl)
	or e
	ld (hl),a
us_next:
	ld hl,(ui_dst)
	ld bc,UI_STRIDE
	add hl,bc
	ld (ui_dst),hl
	ld hl,ui_cnt
	dec (hl)
	jr nz,us_row
	jp ut_after

; One glyph of up to eight columns shifted right by s bits, seven rows. DE = glyph rows,
; HL = first byte in the frame, B = 0. Ends in a jump to ut_after. The ONE
; forms are for a glyph that ends inside the first byte, whose rotated-out bits are blank.
#macro ROW(s)
#if s == 0
	ld a,(de)
	inc de
	or (hl)
	ld (hl),a
	add hl,bc
#else
	ld a,(de)
	inc de
	.fill s,$0F
	ld c,a
	and $FF>>s
	or (hl)
	ld (hl),a
	inc hl
	ld a,c
	and ($FF<<(8-s))&$FF
	or (hl)
	ld (hl),a
	ld c,UI_STRIDE-1
	add hl,bc
#endif
#endmacro
#macro ROW1(s)
	ld a,(de)
	inc de
	.fill s,$0F
	or (hl)
	ld (hl),a
	add hl,bc
#endmacro
#macro VARIANT(s)
#if s == 0
	ld c,UI_STRIDE
#endif
	ROW(s)
	ROW(s)
	ROW(s)
	ROW(s)
	ROW(s)
	ROW(s)
	ROW(s)
	jp ut_after
#endmacro
#macro VARIANT1(s)
	ld c,UI_STRIDE
	ROW1(s)
	ROW1(s)
	ROW1(s)
	ROW1(s)
	ROW1(s)
	ROW1(s)
	ROW1(s)
	jp ut_after
#endmacro
ui_v0:	VARIANT(0)
ui_v1:	VARIANT(1)
ui_v2:	VARIANT(2)
ui_v3:	VARIANT(3)
ui_v4:	VARIANT(4)
ui_v5:	VARIANT(5)
ui_v6:	VARIANT(6)
ui_v7:	VARIANT(7)
ui_w1:	VARIANT1(1)
ui_w2:	VARIANT1(2)
ui_w3:	VARIANT1(3)
ui_w4:	VARIANT1(4)
ui_w5:	VARIANT1(5)
ui_w6:	VARIANT1(6)
ui_w7:	VARIANT1(7)
	.fill (32-($&31))&31,0
ui_vtab:
	.dw ui_v0,ui_v1,ui_v2,ui_v3,ui_v4,ui_v5,ui_v6,ui_v7
	.dw ui_v0,ui_w1,ui_w2,ui_w3,ui_w4,ui_w5,ui_w6,ui_w7
