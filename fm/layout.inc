; fm_rows and fm_draw: lay the loaded tokens out in lines the way mcglyph's place does, and draw the
; lines inside a window.

; in: D = width in pixels
rows_body:
	call fm_need
	jp nz,fm_fail
	ld a,d
	ld (fv_limit),a
	xor a
	ld (fv_draw),a
	call fm_lay
	ld a,(fv_line)
	ld b,a
	xor a
	jp fm_leave

; in: HL = the text row of the first line, signed, from -16384 to 16383; D = width in pixels,
; E = x of the first column, B = the window's first text row, C = one past its last
draw_body:
	call fm_need
	jp nz,fm_fail
	ld a,d
	ld (fv_limit),a
	ld a,e
	ld (fv_x0),a
	ld (fv_row),hl
	ld a,b
	ld (fv_lo),a
	ld a,c
	ld (fv_hi),a
	ld a,1
	ld (fv_draw),a
	call fm_lay
	xor a
	ld b,a
	jp fm_leave

; Places every token, one line at a time, and draws the ones on a visible line when fv_draw is set.
; A token goes on the current line when the line's width, its gap and its own width fit the
; width, else it starts a line. L = the token's index, D = the width of the line so far, E = the
; token count, B = the token's width, C = its column in the line.
fm_lay:
	xor a
	ld (fv_line),a
	ld (fv_show),a
	ld a,(fv_count)
	or a
	ret z
	ld e,a
	ld l,0
	ld h,FM_WIDTHS>>8
	ld b,(hl)
	jr fy_new
fy_next:
	ld a,e
	cp l
	ret z
	ld h,FM_WIDTHS>>8
	ld b,(hl)
	inc h
	ld c,(hl)
	ld a,d
	add a,c
	jr c,fy_new
	add a,b
	jr c,fy_new
	ld h,a
	ld a,(fv_limit)
	cp h
	jr c,fy_new
	ld d,h
	ld a,h
	sub b
	ld c,a
	jr fy_put
fy_new:
	ld d,b
	ld c,0
	ld a,(fv_line)
	inc a
	ld (fv_line),a
	ld a,(fv_draw)
	or a
	jr z,fy_put
	call fy_window
	ret c
fy_put:
	ld a,(fv_show)
	or a
	jr z,fy_step
	push hl
	push de
	push bc
	ld h,FM_IDS>>8
	ld a,(hl)
	call fm_codes
	pop bc
	ld a,(fv_x0)
	add a,c
	ld e,a
	ld d,0
	jr nc,fy_text
	inc d
fy_text:
	ld a,(fv_rowb)
	ld b,a
	call ui_text
	call fm_restore
	pop de
	pop hl
fy_step:
	inc l
	jr fy_next

; The line just started, number fv_line - 1: fv_show says whether it is inside the window and
; fv_rowb is its text row. out: carry when it and every later line is below the window. Keeps
; every register but AF.
fy_window:
	push hl
	push de
	push bc
	ld a,(fv_line)
	dec a
	ld e,a
	ld d,0
	ld hl,(fv_row)
	add hl,de
	ld a,(fv_hi)
	ld e,a
	push hl
	or a
	sbc hl,de
	pop hl
	jp m,fw_before_end
	xor a
	ld (fv_show),a
	scf
	jr fw_done
fw_before_end:
	ld a,(fv_lo)
	ld e,a
	push hl
	or a
	sbc hl,de
	pop hl
	ld a,0
	jp m,fw_set
	ld a,l
	ld (fv_rowb),a
	ld a,1
fw_set:
	ld (fv_show),a
	or a
fw_done:
	pop bc
	pop de
	pop hl
	ret
