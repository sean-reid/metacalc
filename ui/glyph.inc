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

; GLYPH_ADV   in:  A = a glyph code, not 0
;             out: A = pixels the glyph advances: its width and the blank column after it
;             destroys F BC HL; ui_text_width and ui/fit.inc expand it
#macro GLYPH_ADV
	ld l,a
	ld h,ui_toff>>8
	ld c,(hl)
	inc h
	ld b,(hl)
	ld hl,(ui_fontp)
	add hl,bc
	ld a,(hl)
	and $7F
	inc a
#endmacro

; ui_text_width   in:  HL = string
;                 out: HL = width in pixels
;                 destroys AF BC DE
ui_text_width:
	ld de,0
tw_loop:
	ld a,(hl)
	inc hl
	or a
	jr z,tw_done
	push hl
	GLYPH_ADV
	pop hl
	add a,e
	ld e,a
	jr nc,tw_loop
	inc d
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
;                   draws it ending at column 94, from 0 when it is wider, marking row B as ui_text does
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
;                   starts at (96 - width) / 2, or 0 when it is wider, marking row B as ui_text does
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
;           right of column 95. Marks row B in ui_dirty for a non-empty string and a row below 8,
;           also when every pixel falls right of column 95; marks nothing for an empty string or a
;           row of 8 or more. Destroys AF BC HL.
ui_text:
	ld a,(hl)
	or a
	ret z
	ld a,b
	cp UI_TEXT_ROWS
	jp nc,ut_off
	add a,a
	add a,a
	or ut_rows&$FF ; ut_rows is 32 byte aligned, so the offset only sets low bits
	ld c,a
	ld b,ut_rows>>8
	ld a,(bc)
	ld (ui_rowbase),a
	inc c
	ld a,(bc)
	ld (ui_rowbase+1),a
	inc c
	ld a,(bc)
	ld b,a
	ld a,(ui_dirty)
	or b
	ld (ui_dirty),a
	ld a,d
	or a
	jp nz,ut_tail
	ld a,e
	cp UI_WIDTH
	jp nc,ut_tail
	jp ut_loop

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
ut_odd:
	or a
	jr z,ut_end
	push hl
	ld hl,(ui_fontp)
	ld bc,SLOT
	add hl,bc
	ld c,(hl)
	pop hl
	ld a,e
	add a,c
	inc a
	cp UI_WIDTH
	jr nc,ut_clip
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
	cp SPACE_CODE+1
	jr c,ut_odd
	push hl
	ld l,a
	ld h,ui_toff>>8
	ld c,(hl)
	inc h
	ld b,(hl)
	ld hl,(ui_fontp)
	add hl,bc
	ld c,(hl)
	inc hl
	bit 7,c
	jr nz,ut_tall
	ex de,hl
	ld a,l
	add a,c
	inc a
	push af
	ld h,ui_tfit>>8
	ld a,(hl)
	cp c
	jr c,ut_two
	inc h
	ld b,(hl)
	ld c,$80
ut_go:
	push bc
	inc h
	ld c,(hl)
	ld b,0
	ld hl,(ui_rowbase)
	add hl,bc
	ret
ut_two:
	ld a,c
	cp 9
	jr nc,ut_back
	ld a,l
	cp 88
	jr nc,ut_back
	inc h
	ld a,(hl)
	add a,8
	ld b,a
	ld c,$80
	jr ut_go
ut_back:
	ex de,hl
	jr ut_slow
ut_tall:
	res 7,c
	ex de,hl
	ld a,l
	add a,c
	inc a
	push af
	ld h,ui_tfit>>8
	ld a,(hl)
	cp c
	jr c,ut_two_tall
	inc h
	ld b,(hl)
	ld c,0
	jr ut_go
ut_two_tall:
	ld a,c
	cp 9
	jr nc,ut_back
	ld a,l
	cp 88
	jr nc,ut_back
	inc h
	ld a,(hl)
	add a,8
	ld b,a
	ld c,0
	jr ut_go

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

; One glyph of up to eight columns shifted right by s bits. DE = glyph rows, HL = first byte in
; the frame, B = 0. The ui_v entries draw rows 0 to 4 and end in a jump to ut_after; the ui_tv
; entries draw rows 5 and 6 first, for a glyph with a pixel there, then the same five rows. The
; ONE forms are for a glyph that ends inside the first byte, whose rotated-out bits are blank.
; Each form of each shift sits in a 256 byte block of ui_vblk with its ui_tv entry at offset 0 and
; its ui_v entry at offset 80h: blocks 0 to 7 are the ONE forms by shift, blocks 9 to 15 the two
; byte forms of shifts 1 to 7.
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
#if s < 5
	.fill s,$0F
#else
	.fill 8-s,$07
#endif
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
#if s < 5
	.fill s,$0F
#else
	.fill 8-s,$07
#endif
	or (hl)
	ld (hl),a
	add hl,bc
#endmacro
#macro TALL_IN
	ld c,5*UI_STRIDE
	add hl,bc
	ex de,hl
	ld c,5
	add hl,bc
	ex de,hl
#endmacro
#macro TALL_OUT
	ld bc,-7*UI_STRIDE
	add hl,bc
	ex de,hl
	ld bc,-7
	add hl,bc
	ex de,hl
	ld bc,UI_STRIDE
#endmacro
#macro TAIL(s)
#if s == 0
	ld c,UI_STRIDE
#endif
	ROW(s)
	ROW(s)
	ROW(s)
	ROW(s)
	ROW(s)
	jp ut_after
#endmacro
#macro TAIL1(s)
	ld c,UI_STRIDE
	ROW1(s)
	ROW1(s)
	ROW1(s)
	ROW1(s)
	ROW1(s)
	jp ut_after
#endmacro
#macro HEAD(s)
	TALL_IN
#if s == 0
	ld c,UI_STRIDE
#endif
	ROW(s)
	ROW(s)
	TALL_OUT
#endmacro
#macro HEAD1(s)
	TALL_IN
	ld c,UI_STRIDE
	ROW1(s)
	ROW1(s)
	TALL_OUT
#endmacro
	.fill (256-($&255))&255,0
ui_vblk:
#if (ui_vblk&255) != 0
 .error "ui_vblk must be 256 byte aligned"
#endif
ui_tv0:	HEAD(0)
	jr ui_v0
	.fill (128-($&127))&127,0
ui_v0:	TAIL(0)
#if (ui_v0&255) != $80
 .error "a jump block's head must fit below its tail"
#endif
#if (ui_tv0-ui_vblk) != 0
 .error "one byte form block ui_tv0 is not at ui_vblk+0*256: a tail or head outgrew its 256 bytes"
#endif
	.fill (256-($&255))&255,0
ui_tw1:	HEAD1(1)
	jr ui_w1
	.fill (128-($&127))&127,0
ui_w1:	TAIL1(1)
#if (ui_w1&255) != $80
 .error "a jump block's head must fit below its tail"
#endif
#if (ui_tw1-ui_vblk) != 256
 .error "one byte form block ui_tw1 is not at ui_vblk+1*256: a tail or head outgrew its 256 bytes"
#endif
	.fill (256-($&255))&255,0
ui_tw2:	HEAD1(2)
	jr ui_w2
	.fill (128-($&127))&127,0
ui_w2:	TAIL1(2)
#if (ui_w2&255) != $80
 .error "a jump block's head must fit below its tail"
#endif
#if (ui_tw2-ui_vblk) != 512
 .error "one byte form block ui_tw2 is not at ui_vblk+2*256: a tail or head outgrew its 256 bytes"
#endif
	.fill (256-($&255))&255,0
ui_tw3:	HEAD1(3)
	jr ui_w3
	.fill (128-($&127))&127,0
ui_w3:	TAIL1(3)
#if (ui_w3&255) != $80
 .error "a jump block's head must fit below its tail"
#endif
#if (ui_tw3-ui_vblk) != 768
 .error "one byte form block ui_tw3 is not at ui_vblk+3*256: a tail or head outgrew its 256 bytes"
#endif
	.fill (256-($&255))&255,0
ui_tw4:	HEAD1(4)
	jr ui_w4
	.fill (128-($&127))&127,0
ui_w4:	TAIL1(4)
#if (ui_w4&255) != $80
 .error "a jump block's head must fit below its tail"
#endif
#if (ui_tw4-ui_vblk) != 1024
 .error "one byte form block ui_tw4 is not at ui_vblk+4*256: a tail or head outgrew its 256 bytes"
#endif
	.fill (256-($&255))&255,0
ui_tw5:	HEAD1(5)
	jr ui_w5
	.fill (128-($&127))&127,0
ui_w5:	TAIL1(5)
#if (ui_w5&255) != $80
 .error "a jump block's head must fit below its tail"
#endif
#if (ui_tw5-ui_vblk) != 1280
 .error "one byte form block ui_tw5 is not at ui_vblk+5*256: a tail or head outgrew its 256 bytes"
#endif
	.fill (256-($&255))&255,0
ui_tw6:	HEAD1(6)
	jr ui_w6
	.fill (128-($&127))&127,0
ui_w6:	TAIL1(6)
#if (ui_w6&255) != $80
 .error "a jump block's head must fit below its tail"
#endif
#if (ui_tw6-ui_vblk) != 1536
 .error "one byte form block ui_tw6 is not at ui_vblk+6*256: a tail or head outgrew its 256 bytes"
#endif
	.fill (256-($&255))&255,0
ui_tw7:	HEAD1(7)
	jr ui_w7
	.fill (128-($&127))&127,0
ui_w7:	TAIL1(7)
#if (ui_w7&255) != $80
 .error "a jump block's head must fit below its tail"
#endif
#if (ui_tw7-ui_vblk) != 1792
 .error "one byte form block ui_tw7 is not at ui_vblk+7*256: a tail or head outgrew its 256 bytes"
#endif
	.fill (256-($&255))&255,0
ui_hole:
#include "progress.inc"
#include "titlebar.inc"
#if ($-ui_hole) > 256
 .error "the code in the empty block outgrew its 256 bytes"
#endif
	.fill 256-($-ui_hole),0
ui_tv1:	HEAD(1)
	jr ui_v1
	.fill (128-($&127))&127,0
ui_v1:	TAIL(1)
#if (ui_v1&255) != $80
 .error "a jump block's head must fit below its tail"
#endif
#if (ui_tv1-ui_vblk) != 2304
 .error "two byte form block ui_tv1 is not at ui_vblk+9*256: a tail or head outgrew its 256 bytes"
#endif
	.fill (256-($&255))&255,0
ui_tv2:	HEAD(2)
	jr ui_v2
	.fill (128-($&127))&127,0
ui_v2:	TAIL(2)
#if (ui_v2&255) != $80
 .error "a jump block's head must fit below its tail"
#endif
#if (ui_tv2-ui_vblk) != 2560
 .error "two byte form block ui_tv2 is not at ui_vblk+10*256: a tail or head outgrew its 256 bytes"
#endif
	.fill (256-($&255))&255,0
ui_tv3:	HEAD(3)
	jr ui_v3
	.fill (128-($&127))&127,0
ui_v3:	TAIL(3)
#if (ui_v3&255) != $80
 .error "a jump block's head must fit below its tail"
#endif
#if (ui_tv3-ui_vblk) != 2816
 .error "two byte form block ui_tv3 is not at ui_vblk+11*256: a tail or head outgrew its 256 bytes"
#endif
	.fill (256-($&255))&255,0
ui_tv4:	HEAD(4)
	jr ui_v4
	.fill (128-($&127))&127,0
ui_v4:	TAIL(4)
#if (ui_v4&255) != $80
 .error "a jump block's head must fit below its tail"
#endif
#if (ui_tv4-ui_vblk) != 3072
 .error "two byte form block ui_tv4 is not at ui_vblk+12*256: a tail or head outgrew its 256 bytes"
#endif
	.fill (256-($&255))&255,0
ui_tv5:	HEAD(5)
	jr ui_v5
	.fill (128-($&127))&127,0
ui_v5:	TAIL(5)
#if (ui_v5&255) != $80
 .error "a jump block's head must fit below its tail"
#endif
#if (ui_tv5-ui_vblk) != 3328
 .error "two byte form block ui_tv5 is not at ui_vblk+13*256: a tail or head outgrew its 256 bytes"
#endif
	.fill (256-($&255))&255,0
ui_tv6:	HEAD(6)
	jr ui_v6
	.fill (128-($&127))&127,0
ui_v6:	TAIL(6)
#if (ui_v6&255) != $80
 .error "a jump block's head must fit below its tail"
#endif
#if (ui_tv6-ui_vblk) != 3584
 .error "two byte form block ui_tv6 is not at ui_vblk+14*256: a tail or head outgrew its 256 bytes"
#endif
	.fill (256-($&255))&255,0
ui_tv7:	HEAD(7)
	jr ui_v7
	.fill (128-($&127))&127,0
ui_v7:	TAIL(7)
#if (ui_v7&255) != $80
 .error "a jump block's head must fit below its tail"
#endif
#if (ui_tv7-ui_vblk) != 3840
 .error "two byte form block ui_tv7 is not at ui_vblk+15*256: a tail or head outgrew its 256 bytes"
#endif
	.fill (256-($&255))&255,0
ui_vend:
#if (ui_vend-ui_vblk) != 4096
 .error "the jump blocks do not end at ui_vblk+16*256"
#endif
; Pages indexed by x below 96, on consecutive pages: the room left in the byte, the high byte of
; the ONE form block for the shift, and the byte column. ui_toff is indexed by a glyph code: the
; low bytes of the offsets of the slots on its first page, the high bytes on the next.
#macro BYTES8(v)
	.db v,v,v,v,v,v,v,v
#endmacro
#macro SHIFTS(v)
	.db v,v+1,v+2,v+3,v+4,v+5,v+6,v+7
#endmacro
#macro FITS
	.db 8,7,6,5,4,3,2,1
#endmacro
#macro STEP8(v)
	.db v,v+8,v+16,v+24,v+32,v+40,v+48,v+56
#endmacro
#macro LOW32
	STEP8(0)
	STEP8(64)
	STEP8(128)
	STEP8(192)
#endmacro
#macro HIGH32(v)
	.fill 32,v
#endmacro
#macro PAGE(m)
	m
	m
	m
	m
	m
	m
	m
	m
	m
	m
	m
	m
#endmacro
	.fill (256-($&255))&255,0
ui_toff:
#if (ui_toff&255) != 0
 .error "ui_toff must be 256 byte aligned"
#endif
	LOW32
	LOW32
	LOW32
	LOW32
	LOW32
	LOW32
	LOW32
	LOW32
	HIGH32(0)
	HIGH32(1)
	HIGH32(2)
	HIGH32(3)
	HIGH32(4)
	HIGH32(5)
	HIGH32(6)
	HIGH32(7)
ui_tfit:
	PAGE(FITS)
ui_tfit_tail:
#include "rows.inc"
#if ($-ui_tfit_tail) > 160
 .error "the code in the tail of the ui_tfit page outgrew its 160 bytes"
#endif
	.fill 160-($-ui_tfit_tail),0
ui_tblk:
	PAGE(SHIFTS(ui_vblk>>8))
ui_tblk_tail:
#include "clear.inc"
#include "range.inc"
#if ($-ui_tblk_tail) > 160
 .error "the code in the tail of the ui_tblk page outgrew its 160 bytes"
#endif
	.fill 160-($-ui_tblk_tail),0
ui_tcol:
	BYTES8(0)
	BYTES8(1)
	BYTES8(2)
	BYTES8(3)
	BYTES8(4)
	BYTES8(5)
	BYTES8(6)
	BYTES8(7)
	BYTES8(8)
	BYTES8(9)
	BYTES8(10)
	BYTES8(11)
ui_tcol_tail:
#include "rangeh.inc"
#if ($-ui_tcol_tail) > 160
 .error "the code in the tail of the ui_tcol page outgrew its 160 bytes"
#endif
	.fill 160-($-ui_tcol_tail),0
#if ((ui_toff&255) != 0) || (((ui_toff+256)&255) != 0) || ((ui_tfit&255) != 0) || ((ui_tblk&255) != 0) || ((ui_tcol&255) != 0)
 .error "ui_toff, its high half, ui_tfit, ui_tblk and ui_tcol must each start a page"
#endif
#if (ui_toff+512) != ui_tfit
 .error "ui_tfit must follow the two ui_toff pages"
#endif
#if (ui_tblk-ui_tfit) != 256
 .error "the x tables must be consecutive pages"
#endif
#if (ui_tcol-ui_tblk) != 256
 .error "the x tables must be consecutive pages"
#endif
	.fill (32-($&31))&31,0
ut_rows:
#if (ut_rows&31) != 0
 .error "ut_rows must be 32 byte aligned"
#endif
#macro UT_ROW(r)
	.dw UI_FRAME+UI_STRIDE+(r*UI_ROW*UI_STRIDE)
	.db 1<<r,0
#endmacro
	UT_ROW(0)
	UT_ROW(1)
	UT_ROW(2)
	UT_ROW(3)
	UT_ROW(4)
	UT_ROW(5)
	UT_ROW(6)
	UT_ROW(7)
