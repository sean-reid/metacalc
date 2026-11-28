; Row ranges: clearing text rows and moving them by one row.

; ui_clear_rows   in:  A = first text row, B = count
;                 zeroes rows A to A+B-1 of the frame, all eight scanlines of each, cut at row 7,
;                 and sets their bits in ui_dirty; a count of 0 or a first row of 8 or more
;                 changes and marks nothing. Pushes through the stack as ui_clear does, so
;                 interrupts are off for the call and come back as the caller had them
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
;                 caller had them
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
