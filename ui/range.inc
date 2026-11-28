; Row ranges: clearing text rows and moving them by one row. Assembled in the tail of the ui_tblk page
; (glyph.inc).

; ui_clear_rows   in:  A = first text row, B = count
;                 zeroes rows A to A+B-1 of the frame, all eight scanlines of each, cut at row 7,
;                 and sets their bits in ui_dirty; a count of 0 or a first row of 8 or more
;                 changes and marks nothing. Pushes through the stack as ui_clear does, so
;                 interrupts are off for the call and come back as the caller had them. The stack
;                 needed is 6 bytes, 4 when it changes nothing
;                 destroys AF BC DE HL
ui_clear_rows:
	call ur_clip
	ret z
	call ur_mark
	jp ur_clear

; ui_shift_rows   in:  A = first text row, B = count, C = 0 to move up, anything else down
;                 moves the B rows from A, cut at row 7, by one row: up, rows A+1 to A+B-1 go to
;                 A to A+B-2 and row A+B-1 is zeroed; down, rows A to A+B-2 go to A+1 to A+B-1 and
;                 row A is zeroed. Sets the bits of all B rows in ui_dirty; a count of 0 or a first
;                 row of 8 or more changes and marks nothing, and a count of 1 zeroes the row.
;                 The zeroing is ui_clear_rows's, so interrupts are off for it and come back as the
;                 caller had them. The stack needed is 10 bytes, 6 for a count of 1 and 4 when
;                 it changes nothing
;                 destroys AF BC DE HL
ui_shift_rows:
	ld d,c
	call ur_clip
	ret z
	call ur_mark
	ld a,b
	dec a
	jp z,ur_clear
	push bc
	push de
	ld a,c
	call ur_base
	push hl
	ld a,b
	dec a
	call ur_96
	ld b,h
	ld c,l
	pop hl
	pop de
	ld a,d
	or a
	jr nz,sr_down
	ld d,h
	ld e,l
	ld a,UI_ROW*UI_STRIDE
	add a,l
	ld l,a
	jr nc,sr_up
	inc h
sr_up:
	ldir
	pop bc
	ld a,c
	add a,b
	dec a
	ld c,a
	ld b,1
	jp ur_clear
sr_down:
	add hl,bc
	dec hl
	ld d,h
	ld e,l
	ld a,UI_ROW*UI_STRIDE
	add a,e
	ld e,a
	jr nc,sr_dn
	inc d
sr_dn:
	lddr
	pop bc
	ld b,1
	jp ur_clear
