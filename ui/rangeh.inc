; The helpers of ui_clear_rows and ui_shift_rows. Assembled in the tail of the ui_tcol page (glyph.inc).

; ur_clip   in:  A = first text row, B = count
;           out: Z when there is nothing to do, a first row of 8 or more or a count of 0;
;                else NZ, C = first row, B = count cut at row 7
;           destroys AF
ur_clip:
	cp UI_TEXT_ROWS
	jr nc,ur_none
	ld c,a
	ld a,b
	or a
	ret z
	ld a,UI_TEXT_ROWS
	sub c
	cp b
	jr nc,ur_keep
	ld b,a
ur_keep:
	or a
	ret
ur_none:
	xor a
	ret

; ur_mark   in:  C = first text row, B = count, at least 1, first + count at most 8
;           sets those rows' bits in ui_dirty
;           destroys AF E HL
ur_mark:
	ld e,b
	xor a
ur_ones:
	scf
	rla
	djnz ur_ones
	ld b,c
	inc b
	jr ur_start
ur_sh:
	add a,a
ur_start:
	djnz ur_sh
	ld b,e
	ld hl,ui_dirty
	or (hl)
	ld (hl),a
	ret

; ur_96     in:  A = 0 to 8
;           out: HL = 96 * A, the bytes of that many text rows
;           destroys AF DE
ur_96:
	ld l,a
	ld h,0
	ld e,a
	ld d,h
	add hl,hl
	add hl,de
	add hl,hl
	add hl,hl
	add hl,hl
	add hl,hl
	add hl,hl
	ret

; ur_base   in:  A = 0 to 8
;           out: HL = the frame address where text row A starts, scanline 0
;           destroys AF DE
ur_base:
	call ur_96
	ld de,UI_FRAME
	add hl,de
	ret

; ur_clear  in:  C = first text row, B = count, at least 1, first + count at most 8
;           zeroes those rows through the stack and ends the calling routine, with interrupts as
;           they were at its call
ur_clear:
	ld a,c
	add a,b
	call ur_base
	ld a,b
	add a,a
	add a,b
	ld b,a
	ld a,i
	jp pe,ur_on
	ld a,i
	jp pe,ur_on
	xor a
	jr ur_go
ur_on:
	ld a,1
ur_go:
	ld (ui_iff),a
	di
	ld (ui_sp),sp
	ld sp,hl
	ld hl,0
	jp uc_loop
