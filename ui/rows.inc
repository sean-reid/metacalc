; Rows: the row address, the margin marker and the inverted row. Assembled in the tail of the ui_tfit page (glyph.inc).

; ui_row_addr   in:  A = text row 0 to 7
;               out: HL = frame address of the first byte of the row's first glyph scanline
;               sets the row's bit in ui_dirty: its callers below and ui_bar write the row
;               destroys AF DE
ui_row_addr:
	add a,a
	add a,a
	or ut_rows&$FF
	ld l,a
	ld h,ut_rows>>8
	ld e,(hl)
	inc l
	ld d,(hl)
	inc l
	ld a,(hl)
	ld hl,ui_dirty
	or (hl)
	ld (hl),a
	ex de,hl
	ret

; ui_marker   in:  A = text row 0 to 7, nothing drawn or marked for a larger one
;             sets columns 0 and 1 on the seven scanlines below the row's blank one and marks the
;             row, whether or not those pixels were set already
;             destroys AF BC DE HL
ui_marker:
	cp UI_TEXT_ROWS
	ret nc
	call ui_row_addr
	ld bc,UI_STRIDE
	ld a,7
mk_loop:
	set 7,(hl)
	set 6,(hl)
	add hl,bc
	dec a
	jr nz,mk_loop
	ret

; ui_invert_row   in:  A = text row 0 to 7, nothing drawn or marked for a larger one
;                 flips the seven scanlines of the row below its top one and marks the row
;                 destroys AF BC DE HL
ui_invert_row:
	cp UI_TEXT_ROWS
	ret nc
	call ui_row_addr
	ld b,5
ir_loop:
#macro CPL4
	ld a,(hl)
	cpl
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
	cpl
	ld (hl),a
	inc hl
#endmacro
	CPL4
	CPL4
	CPL4
	CPL4
	djnz ir_loop
	CPL4
	ret
